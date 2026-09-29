//! TypeScript frontend: the assertion sites of a project (`<runtime read>
//! as T`) as one-hop `writes_to` override edges from the enclosing function
//! to the slots of `T`. See `docs/design/typescript-frontend.md`.

pub mod sites;
pub mod types;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_parser::Parser;
use oxc_span::SourceType;

use crate::db::{
    ConstraintType, ContractDb, ContractRecord, ContractRole, Discovery, EdgeRecord, NodeKind,
    NodeRecord, Relationship, VerificationLevel,
};
use crate::source::LineIndex;
use crate::value_analysis::{self, ValueFacts};
use sites::FunctionRef;
use types::{Requirement, Shape, TsConfig, TypeIndex};

/// One parsed file; the AST borrows a per-file arena.
pub struct ParsedModule<'a> {
    /// Path relative to the application root (as recorded in `source_file`).
    pub rel: String,
    /// Absolute path, for import resolution.
    pub abs: PathBuf,
    pub program: Program<'a>,
    pub source: &'a str,
    pub lines: LineIndex,
    /// A `.d.ts`: feeds the type index only.
    pub is_declaration: bool,
    pub is_typescript: bool,
}

/// A model node: one requirement a cast is checked against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slot {
    pub qualified: String,
    pub short: String,
    pub file: String,
    pub line: u32,
    pub requirement: Requirement,
}

/// One edge: `function` asserts the value of a runtime read into `slot`.
#[derive(Debug, Clone)]
pub struct Assertion {
    /// Qualified names.
    pub function: String,
    pub slot: String,
    pub site_file: String,
    pub site_line: u32,
    /// The guarantee on this edge (the override rows).
    pub facts: ValueFacts,
}

/// Everything the frontend extracted, before SQLite.
#[derive(Debug, Default)]
pub struct TsGraph {
    /// Path heads: a function is present iff it encloses a site.
    pub functions: Vec<FunctionRef>,
    pub slots: Vec<Slot>,
    pub assertions: Vec<Assertion>,
    /// `path: line N: message` per file with a syntax error (the file is skipped).
    pub parse_errors: Vec<String>,
    /// Assertion sites found, including those that wrote nothing.
    pub sites: usize,
    /// Sites whose target type is `Unknown`.
    pub skipped_unresolved: usize,
}

/// Rows written by `TsGraph::write`.
#[derive(Debug, Default)]
pub struct TsCounts {
    pub functions: usize,
    pub slots: usize,
    pub edges: usize,
}

/// The frontend over `files` (absolute or root-relative paths under `app_root`).
/// Pure apart from reading the files and the nearest `tsconfig.json`.
pub fn extract(files: &[PathBuf], app_root: &Path) -> Result<TsGraph> {
    if files.is_empty() {
        return Ok(TsGraph::default());
    }
    let root = std::fs::canonicalize(app_root).unwrap_or_else(|_| app_root.to_path_buf());
    let mut sources = Vec::with_capacity(files.len());
    for f in files {
        let bytes = std::fs::read(f)?;
        sources.push((
            crate::extractor::relative_path(app_root, f),
            String::from_utf8_lossy(&bytes).into_owned(),
        ));
    }
    let config = TsConfig::load(&root);
    Ok(extract_sources(sources, &root, &config))
}

/// The frontend over `(relative path, source)` pairs under `root`.
pub fn extract_sources(sources: Vec<(String, String)>, root: &Path, config: &TsConfig) -> TsGraph {
    let allocators: Vec<Allocator> = sources.iter().map(|_| Allocator::default()).collect();
    let mut modules: Vec<ParsedModule> = Vec::with_capacity(sources.len());
    let mut parse_errors = Vec::new();
    for ((rel, source), allocator) in sources.iter().zip(&allocators) {
        let source_type = SourceType::from_path(rel).unwrap_or_else(|_| SourceType::ts());
        // Plain `.js` files may hold JSX (`allowJs` projects).
        let source_type = source_type.with_jsx(source_type.is_javascript() || source_type.is_jsx());
        let parsed = Parser::new(allocator, source, source_type).parse();
        let lines = LineIndex::new(source);
        if let Some(e) = parsed.diagnostics.errors().next() {
            let line = e.labels.first().map_or(1, |l| lines.line(l.offset()));
            parse_errors.push(format!("{rel}: line {line}: {}", e.message));
            continue;
        }
        modules.push(ParsedModule {
            rel: rel.clone(),
            abs: types::normalize(&root.join(rel)),
            program: parsed.program,
            source,
            lines,
            is_declaration: source_type.is_typescript_definition(),
            is_typescript: source_type.is_typescript(),
        });
    }

    let index = TypeIndex::build(&modules, config);
    let mut graph = TsGraph { parse_errors, ..Default::default() };
    let mut slot_ids: HashMap<String, usize> = HashMap::new();
    for (fid, m) in modules.iter().enumerate() {
        if !m.is_typescript || m.is_declaration {
            continue;
        }
        for site in sites::collect(&index, fid, m) {
            graph.sites += 1;
            let rel_of = |file: usize| modules[file].rel.as_str();
            let target = &site.target;
            let shape = match &target.shape {
                Shape::Nullable(inner) if matches!(**inner, Shape::Object(_)) => &**inner,
                s => s,
            };
            let candidates: Vec<(Slot, ValueFacts)> = match shape {
                Shape::Unknown => {
                    graph.skipped_unresolved += 1;
                    continue;
                }
                Shape::Object(fields) => fields
                    .iter()
                    .map(|f| {
                        let slot = Slot {
                            qualified: format!("{}.{}.{}", rel_of(target.file), target.name, f.name),
                            short: format!("{}.{}", target.name, f.name),
                            file: rel_of(f.file).to_string(),
                            line: f.line,
                            requirement: f.shape.requirement(f.optional),
                        };
                        let facts = site.guard.get(&f.name).cloned().unwrap_or_default();
                        (slot, facts)
                    })
                    .collect(),
                shape => vec![(
                    Slot {
                        qualified: format!("{}.{}", rel_of(target.file), target.name),
                        short: target.name.clone(),
                        file: rel_of(target.file).to_string(),
                        line: target.line,
                        requirement: shape.requirement(false),
                    },
                    site.source.facts(),
                )],
            };
            if !graph.functions.contains(&site.function) {
                graph.functions.push(site.function.clone());
            }
            for (slot, facts) in candidates {
                if !slot.requirement.can_reject() {
                    continue;
                }
                let qualified = slot.qualified.clone();
                if !slot_ids.contains_key(&qualified) {
                    slot_ids.insert(qualified.clone(), graph.slots.len());
                    graph.slots.push(slot);
                }
                graph.assertions.push(Assertion {
                    function: site.function.qualified.clone(),
                    slot: qualified,
                    site_file: m.rel.clone(),
                    site_line: site.line,
                    facts,
                });
            }
        }
    }
    graph
}

impl TsGraph {
    /// `(qualified, short)` pairs of every node, for `resolve::display_names`.
    pub fn node_names(&self) -> Vec<(String, String)> {
        self.functions
            .iter()
            .map(|f| (f.qualified.clone(), f.short.clone()))
            .chain(self.slots.iter().map(|s| (s.qualified.clone(), s.short.clone())))
            .collect()
    }

    /// Write function nodes (no rows), slot nodes with their precondition
    /// rows, and one `writes_to` override edge per assertion.
    pub fn write(&self, db: &ContractDb, names: &HashMap<String, String>) -> Result<TsCounts> {
        let display = |q: &str, short: &str| names.get(q).cloned().unwrap_or_else(|| short.to_string());
        let mut ids: HashMap<&str, i64> = HashMap::new();
        for f in &self.functions {
            let id = db.insert_node(&NodeRecord {
                name: display(&f.qualified, &f.short),
                qualified_name: Some(f.qualified.clone()),
                kind: NodeKind::Function,
                source_file: f.file.clone(),
                source_line: f.line,
                is_call_site: false,
            })?;
            ids.insert(&f.qualified, id);
        }
        for s in &self.slots {
            let id = db.insert_node(&NodeRecord {
                name: display(&s.qualified, &s.short),
                qualified_name: Some(s.qualified.clone()),
                kind: NodeKind::Model,
                source_file: s.file.clone(),
                source_line: s.line,
                is_call_site: false,
            })?;
            ids.insert(&s.qualified, id);
            let row = |kind: ConstraintType| {
                ContractRecord::new(
                    id,
                    kind,
                    ContractRole::Precondition,
                    VerificationLevel::Extracted,
                    &s.file,
                    s.line,
                )
            };
            if let Some(n) = s.requirement.nullable {
                db.insert_contract(&ContractRecord {
                    param_nullable: Some(n as i64),
                    ..row(ConstraintType::Nullability)
                })?;
            }
            if let Some(choices) = &s.requirement.choices {
                db.insert_contract(&row(ConstraintType::Choices).with_choices(choices))?;
            }
        }
        for a in &self.assertions {
            let source_id = ids[a.function.as_str()];
            let edge_id = db.insert_edge(&EdgeRecord {
                source_node_id: source_id,
                target_node_id: ids[a.slot.as_str()],
                relationship: Relationship::WritesTo,
                discovery: Discovery::AstPattern,
                target_param: None,
                source_override: true,
                site_file: Some(a.site_file.clone()),
                site_line: Some(a.site_line),
            })?;
            for mut row in value_analysis::facts_rows(
                &a.facts,
                source_id,
                &a.site_file,
                a.site_line,
                VerificationLevel::Extracted,
            ) {
                row.edge_id = Some(edge_id);
                db.insert_contract(&row)?;
            }
        }
        Ok(TsCounts {
            functions: self.functions.len(),
            slots: self.slots.len(),
            edges: self.assertions.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(files: &[(&str, &str)]) -> TsGraph {
        extract_sources(
            files.iter().map(|(p, s)| (p.to_string(), s.to_string())).collect(),
            Path::new("/app"),
            &TsConfig::parse(r#"{"compilerOptions": {"baseUrl": ".", "paths": {"@/*": ["*"]}}}"#, Path::new("/app")).unwrap(),
        )
    }

    fn slots(g: &TsGraph) -> Vec<String> {
        let mut out: Vec<String> = g
            .slots
            .iter()
            .map(|s| {
                format!(
                    "{} ({}:{}) nullable={:?} choices={:?}",
                    s.short, s.file, s.line, s.requirement.nullable, s.requirement.choices
                )
            })
            .collect();
        out.sort();
        out
    }

    #[test]
    fn interface_slots_across_files_and_tsconfig_paths() {
        let g = graph(&[
            (
                "app/session.ts",
                "export type Strategy = 'oauth_google' | 'oauth_apple';\nexport interface Base { id: number; }",
            ),
            (
                "domain/stash.ts",
                "import type { Strategy } from '@/app/session';
import { Base as B } from '../app/session';
export interface Stash extends B {
  clientName: string;
  origin: 'login' | 'signup';
  search?: string;
  strategy?: Strategy;
  linkPending?: boolean;
}
export const peek = (clientName: string): Stash | null => {
  const raw = sessionStorage.getItem('k');
  if (!raw) return null;
  const stash = JSON.parse(raw) as Stash;
  return stash.clientName === clientName ? stash : null;
};",
            ),
        ]);
        assert_eq!(
            slots(&g),
            vec![
                "Stash.clientName (domain/stash.ts:4) nullable=Some(false) choices=None",
                "Stash.id (app/session.ts:2) nullable=Some(false) choices=None",
                "Stash.origin (domain/stash.ts:5) nullable=Some(false) choices=Some([\"login\", \"signup\"])",
                "Stash.strategy (domain/stash.ts:7) nullable=Some(true) choices=Some([\"oauth_google\", \"oauth_apple\"])",
            ]
        );
        assert_eq!(g.slots[0].qualified, "domain/stash.ts.Stash.id");
        assert_eq!(g.assertions.len(), 4);
        assert!(g.assertions.iter().all(|a| a.site_line == 13 && a.function == "domain/stash.ts.peek@10"));
        assert_eq!(g.skipped_unresolved, 0);
    }

    #[test]
    fn enums_aliases_of_aliases_and_arrays() {
        let g = graph(&[(
            "m.ts",
            "enum Color { Red = 'red', Blue = 'blue' }
enum Level { Low, High = 5, Higher }
type A = 'a' | B;
type B = 'b' | 'c';
type Theme = A | Color;
export function f(raw: string) {
  const a = localStorage.getItem('c') as Color;
  const b = JSON.parse(raw) as Level[];
  const c = JSON.parse(raw) as (Theme | null);
  const d = JSON.parse(raw) as Theme[] | null;
  return [a, b, c, d];
}",
        )]);
        assert_eq!(
            slots(&g),
            vec![
                "Color (m.ts:1) nullable=Some(false) choices=Some([\"red\", \"blue\"])",
                "Level (m.ts:2) nullable=Some(false) choices=Some([\"0\", \"5\", \"6\"])",
                "Theme (m.ts:5) nullable=Some(false) choices=Some([\"a\", \"b\", \"c\", \"red\", \"blue\"])",
                "Theme | null (m.ts:9) nullable=Some(true) choices=Some([\"a\", \"b\", \"c\", \"red\", \"blue\"])",
            ]
        );
    }

    #[test]
    fn unknown_targets_are_counted_not_guessed() {
        let g = graph(&[(
            "m.ts",
            "import { Missing } from './nowhere';
type Loop = Loop | 'a';
type Gen<T> = T | 'a';
interface Obj { k: keyof typeof X; n: number; }
export function f(raw: string) {
  const a = JSON.parse(raw) as Missing;
  const b = JSON.parse(raw) as Loop;
  const c = JSON.parse(raw) as Gen<string>;
  const d = JSON.parse(raw) as Obj;
  const e = JSON.parse(raw) as string & { brand: true };
  const g = JSON.parse(raw) as unknown;
  return [a, b, c, d, e, g];
}",
        )]);
        assert_eq!(g.skipped_unresolved, 4);
        assert_eq!(slots(&g), vec!["Obj.n (m.ts:4) nullable=Some(false) choices=None"]);
        assert_eq!(g.sites, 5);
    }

    #[test]
    fn declaration_files_feed_types_only_and_js_yields_nothing() {
        let g = graph(&[
            ("types.d.ts", "declare interface Cfg { mode: 'a' | 'b' }\nconst x = JSON.parse('') as Cfg;"),
            ("use.ts", "const c = JSON.parse('') as Cfg;"),
            ("legacy.js", "const c = JSON.parse('');"),
        ]);
        assert_eq!(g.sites, 1);
        assert_eq!(slots(&g), vec!["Cfg.mode (types.d.ts:1) nullable=Some(false) choices=Some([\"a\", \"b\"])"]);
        assert_eq!(g.functions[0].short, "<module use.ts>");
    }

    #[test]
    fn a_global_declared_in_two_scripts_is_unknown() {
        let g = graph(&[
            ("a.d.ts", "interface Cfg { mode: 'a' }"),
            ("b.d.ts", "interface Cfg { mode: 'b' }"),
            ("m.d.ts", "export interface Cfg { mode: 'c' }"),
            ("use.ts", "export const c = JSON.parse('') as Cfg;"),
        ]);
        assert_eq!((g.sites, g.skipped_unresolved), (1, 1));
        assert!(g.slots.is_empty());
    }

    #[test]
    fn syntax_errors_are_reported_and_the_file_skipped() {
        let g = graph(&[("bad.ts", "const = ;\n"), ("ok.ts", "type T = 'a';\nconst c = JSON.parse('') as T;")]);
        assert_eq!(g.parse_errors.len(), 1);
        assert!(g.parse_errors[0].starts_with("bad.ts: line 1: "), "{:?}", g.parse_errors);
        assert_eq!(g.sites, 1);
    }
}
