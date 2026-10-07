use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use ruff_python_ast::{Expr, Stmt};

use crate::dataclass_extractor::{self, DataClass};
use crate::db::{
    ContractDb, ContractRecord, Discovery, EdgeRecord, NodeKind, NodeRecord, Relationship,
};
use crate::defaults::{self, FieldDefaults};
use crate::docstring_parser::{self, DocstringContract};
use crate::edge_discovery::{self, CallSite, DiscoveredEdge, SiteKey};
use crate::exits;
use crate::function_extractor;
use crate::model_extractor::{self, ModelField};
use crate::resolve::{self, ClassInfo, ClassKind, EnumKind, ModuleInfo, ProjectIndex, Symbol};
use crate::source::LineIndex;
use crate::value_analysis::{self, ValueFacts};

/// One parsed Python file.
pub struct SourceModule {
    /// Path relative to the application root (as recorded in `source_file`).
    pub relative_path: String,
    /// Dotted module name.
    pub module: String,
    pub is_package: bool,
    pub source: String,
    pub stmts: Vec<Stmt>,
    pub lines: LineIndex,
    /// The first syntax error, as `line N: message`.
    pub parse_error: Option<String>,
}

impl SourceModule {
    pub fn parse(relative_path: &str, source: String) -> Self {
        let parsed =
            ruff_python_parser::parse_unchecked(&source, ruff_python_parser::Mode::Module.into());
        let lines = LineIndex::new(&source);
        let parse_error = parsed.errors().first().map(|e| {
            use ruff_text_size::Ranged;
            format!("line {}: {}", lines.line(e.range().start().to_u32()), e.error)
        });
        let stmts = match parsed.into_syntax() {
            ruff_python_ast::Mod::Module(module) => module.body.to_vec(),
            _ => Vec::new(),
        };
        SourceModule {
            relative_path: relative_path.to_string(),
            module: resolve::module_name(relative_path),
            is_package: relative_path.ends_with("__init__.py"),
            source,
            stmts,
            lines,
            parse_error,
        }
    }

    /// A module from already parsed statements (no source text).
    pub fn from_stmts(relative_path: &str, module: &str, stmts: Vec<Stmt>) -> Self {
        SourceModule {
            relative_path: relative_path.to_string(),
            module: module.to_string(),
            is_package: false,
            source: String::new(),
            stmts,
            lines: LineIndex::default(),
            parse_error: None,
        }
    }
}

/// Everything extracted from a project, before it is written to SQLite.
pub struct Project {
    pub modules: Vec<SourceModule>,
    /// Relative path → index into `modules`.
    module_ids: HashMap<String, usize>,
    pub index: ProjectIndex,
    pub model_fields: Vec<ModelField>,
    pub data_classes: Vec<DataClass>,
    /// Return-value summaries (EXTRACTED facts) per qualified function name.
    pub summaries: HashMap<String, ValueFacts>,
    /// Docstring clauses per qualified function name.
    pub docstrings: HashMap<String, Vec<DocstringContract>>,
}

impl Project {
    /// Parse and analyse `(relative path, source)` pairs with the Django 4.2 defaults.
    pub fn from_sources(files: Vec<(String, String)>) -> Project {
        let modules = files
            .into_iter()
            .map(|(p, s)| SourceModule::parse(&p, s))
            .collect();
        Project::from_parsed(modules)
    }

    /// Analyse parsed modules with the Django 4.2 defaults.
    pub fn from_parsed(modules: Vec<SourceModule>) -> Project {
        let defaults = defaults::load_defaults("4.2").unwrap_or_default();
        Project::build(modules, &defaults)
    }

    pub fn build(
        modules: Vec<SourceModule>,
        field_defaults: &HashMap<String, FieldDefaults>,
    ) -> Project {
        let mut index = ProjectIndex::default();
        let mut candidates = Vec::new();

        for m in &modules {
            index.modules.insert(
                m.module.clone(),
                ModuleInfo::from_stmts(&m.module, m.is_package, &m.stmts),
            );
        }
        let module_stmts: Vec<(String, &[Stmt])> = modules
            .iter()
            .map(|m| (m.module.clone(), m.stmts.as_slice()))
            .collect();
        let project_aliases = resolve::project_aliases(&module_stmts, &index);
        let mut mutated = std::collections::HashSet::new();
        for m in &modules {
            index
                .stored_attributes
                .extend(resolve::stored_attribute_names(&m.stmts));
            mutated.extend(resolve::mutated_container_names(&m.stmts));
        }
        for m in &modules {
            for (name, value) in resolve::module_const_dicts(&m.stmts) {
                let defined = index.modules[&m.module].defs.contains_key(&name)
                    || index.modules[&m.module].imports.contains_key(&name);
                if !mutated.contains(&name) && !defined {
                    index.const_dicts.insert(resolve::qualify(&m.module, &name), value);
                }
            }
        }
        for m in &modules {
            for name in resolve::module_patterns(&m.stmts) {
                index.patterns.insert(resolve::qualify(&m.module, &name));
            }
            for (name, value) in resolve::module_constants(&m.stmts) {
                if index.modules[&m.module].defs.get(&name) == Some(&resolve::DefKind::Constant) {
                    index
                        .constants
                        .insert(resolve::qualify(&m.module, &name), value);
                }
            }

            // Functions (a later definition with the same qualified name replaces an earlier one)
            let module_fn =
                function_extractor::module_function(&m.stmts, &m.relative_path, &m.module);
            let no_aliases = HashMap::new();
            let aliases = project_aliases.get(&m.module).unwrap_or(&no_aliases);
            let external_types = resolve::external_type_names(&index, &m.module, aliases);
            for mut func in
                function_extractor::extract_functions(&m.stmts, &m.relative_path, &m.module)
                    .into_iter()
                    .chain(module_fn)
            {
                func.source_line = m.lines.line(func.source_line);
                func.return_line = m.lines.line(func.return_line);
                function_extractor::apply_aliases(&mut func, aliases);
                function_extractor::apply_external_types(&mut func, &external_types);
                function_extractor::apply_shadows(&mut func, &|a| {
                    index.annotation_shadow(&m.module, dataclass_extractor::unalias(a, aliases))
                });
                match index.function_ids.get(&func.qualified_name) {
                    Some(&i) => index.functions[i] = func,
                    None => {
                        index
                            .function_ids
                            .insert(func.qualified_name.clone(), index.functions.len());
                        index.functions.push(func);
                    }
                }
            }

            // Classes (module level, and nested in class bodies as `Outer.Inner`)
            for c in resolve::module_classes(&m.stmts) {
                let nested = resolve::nested_classes(c)
                    .into_iter()
                    .map(|(path, inner)| (format!("{}.{path}", c.name), inner));
                for (name, class) in std::iter::once((c.name.to_string(), c)).chain(nested) {
                    let bases = class
                        .arguments
                        .as_ref()
                        .map(|a| a.args.to_vec())
                        .unwrap_or_default();
                    let mut info = ClassInfo::new(&m.module, &name, bases).with_body(&class.body);
                    // A metaclass keyword or a class decorator may change how members are made.
                    let plain_decorators = class.decorator_list.iter().all(|d| {
                        let target = match &d.expression {
                            Expr::Call(c) => c.func.as_ref(),
                            other => other,
                        };
                        resolve::dotted_parts(target)
                            .is_some_and(|p| p.last().is_some_and(|l| l == "unique" || l == "verify"))
                    });
                    // A method decorator of an enum body must be the builtin.
                    let info_m = &index.modules[&m.module];
                    let decorated = class
                        .body
                        .iter()
                        .any(|s| matches!(s, Stmt::FunctionDef(f) if !f.decorator_list.is_empty()));
                    let builtins_shadowed = !info_m.star_imports.is_empty()
                        || resolve::ENUM_METHOD_DECORATORS.iter().any(|d| {
                            info_m.defs.contains_key(*d)
                                || info_m.imports.contains_key(*d)
                                || info_m.rebound.contains(*d)
                                || info.body_names.contains(*d)
                        });
                    if !plain_decorators
                        || class.arguments.as_ref().is_some_and(|a| !a.keywords.is_empty())
                        || (decorated && builtins_shadowed)
                    {
                        info.enum_members = None;
                    }
                    index.classes.insert(info.qualified.clone(), info);
                }
            }

            candidates.extend(
                dataclass_extractor::collect_classes_with(&m.stmts, &m.source, &m.relative_path, aliases)
                    .into_iter()
                    .map(|mut c| {
                        c.external_types = external_types.clone();
                        c.type_shadows = dataclass_extractor::field_annotations(&c.class_def)
                            .filter_map(|(field, a)| {
                                let a = dataclass_extractor::unalias(a, aliases);
                                Some((field, index.annotation_shadow(&m.module, a)?))
                            })
                            .collect();
                        c
                    }),
            );
        }

        for func in &index.functions {
            if let Some(class) = &func.class_name {
                let q = resolve::qualify(&func.module, class);
                if let Some(c) = index.classes.get_mut(&q) {
                    let method = func
                        .name
                        .rsplit('.')
                        .next()
                        .unwrap_or(&func.name)
                        .to_string();
                    c.methods.insert(method, func.qualified_name.clone());
                }
            }
        }

        index.index_subclasses();
        let model_fields = django_models(&modules, &mut index, field_defaults);

        let mut data_classes = {
            let idx = &index;
            dataclass_extractor::resolve_data_classes_with(&candidates, &|c, base| match idx
                .resolve_expr(&c.module, base)
            {
                Some(Symbol::Class(q)) => Some(q),
                _ => None,
            })
        };
        enum_field_choices(&mut data_classes, &index);
        let field_facts: Vec<HashMap<String, value_analysis::ValueFacts>> = data_classes
            .iter()
            .map(|dc| {
                dc.fields
                    .iter()
                    .map(|f| {
                        let mut facts = dataclass_extractor::requirement_facts(&index, dc, f);
                        // A lax pydantic numeric field accepts other numbers
                        // but stores the declared type (it converts).
                        if facts.type_name.is_none() && dc.lax_numeric(f) {
                            facts.type_name = f
                                .type_name
                                .clone()
                                .filter(|t| matches!(t.as_str(), "Decimal" | "int" | "float"));
                        }
                        (f.field_name.clone(), facts)
                    })
                    .collect()
            })
            .collect();
        for (dc, field_facts) in data_classes.iter().zip(field_facts) {
            if let Some(c) = index.classes.get_mut(&dc.qualified_name) {
                if c.kind == ClassKind::Plain {
                    c.kind = ClassKind::Data(dc.kind);
                    c.fields = dc.fields.iter().map(|f| f.field_name.clone()).collect();
                    c.field_nullable = dc
                        .fields
                        .iter()
                        .filter_map(|f| Some((f.field_name.clone(), f.nullable?)))
                        .collect();
                    c.positional = dc.positional.clone();
                    c.strict = dc.strict;
                    if dc.kind == dataclass_extractor::DataClassKind::Pydantic {
                        c.validate_checked = dc
                            .fields
                            .iter()
                            .filter(|f| !f.validation_enforces())
                            .map(|f| f.field_name.clone())
                            .collect();
                    }
                    c.field_facts = field_facts;
                }
            }
        }

        let docstrings = index
            .functions
            .iter()
            .filter_map(|f| {
                let doc = docstring_parser::parse_docstring(f.docstring.as_deref()?);
                (!doc.is_empty()).then(|| (f.qualified_name.clone(), doc))
            })
            .collect();

        let exits: Vec<_> = index
            .functions
            .iter()
            .map(|f| exits::function_exits(&index, f))
            .collect();
        for (f, e) in index.functions.iter_mut().zip(exits) {
            f.exits = e;
        }

        let summaries = value_analysis::compute_summaries(&index);

        let module_ids = modules
            .iter()
            .enumerate()
            .map(|(i, m)| (m.relative_path.clone(), i))
            .collect();
        Project {
            modules,
            module_ids,
            index,
            model_fields,
            data_classes,
            summaries,
            docstrings,
        }
    }

    /// Source text of a module by its relative path.
    pub fn source_of(&self, relative_path: &str) -> &str {
        self.module_ids
            .get(relative_path)
            .map(|&i| self.modules[i].source.as_str())
            .unwrap_or("")
    }

    /// Line index of a module by its relative path.
    pub fn lines_of(&self, relative_path: &str) -> Option<&LineIndex> {
        self.module_ids
            .get(relative_path)
            .map(|&i| &self.modules[i].lines)
    }

    /// `(qualified, short)` names of every node, for display-name selection.
    fn node_names(&self) -> Vec<(String, String)> {
        let mut seen = HashSet::new();
        let mut names = Vec::new();
        let mut add = |q: String, short: String| {
            if seen.insert(q.clone()) {
                names.push((q, short));
            }
        };
        for f in &self.model_fields {
            add(
                model_extractor::field_qualified(f),
                format!("{}.{}", f.model_name, f.field_name),
            );
        }
        for c in &self.data_classes {
            for f in &c.fields {
                add(
                    dataclass_extractor::field_qualified(f),
                    format!("{}.{}", f.class_name, f.field_name),
                );
            }
        }
        for f in &self.index.functions {
            add(f.qualified_name.clone(), f.name.clone());
        }
        for f in self.index.functions.iter().filter(|f| f.has_return_contract()) {
            add(f.return_node_name(), format!("{}.<return>", f.name));
        }
        names
    }
}

/// Recognise Django models (bases naming `models.Model`, or project classes
/// that are models, across modules), mark them in the index, and return
/// their fields: inherited ones first (under the child's name), a child
/// field replacing a parent field of the same name.
fn django_models(
    modules: &[SourceModule],
    index: &mut ProjectIndex,
    field_defaults: &HashMap<String, FieldDefaults>,
) -> Vec<ModelField> {
    // Class definitions by qualified name, in module order.
    let mut defs: Vec<(String, &ruff_python_ast::StmtClassDef, &SourceModule)> = Vec::new();
    for m in modules {
        for c in resolve::module_classes(&m.stmts) {
            defs.push((resolve::qualify(&m.module, c.name.as_str()), c, m));
        }
    }
    let base_classes: HashMap<String, Vec<String>> = defs
        .iter()
        .map(|(q, c, m)| {
            let bases = c
                .arguments
                .as_ref()
                .map(|a| {
                    a.args
                        .iter()
                        .filter_map(|b| match index.resolve_expr(&m.module, b) {
                            Some(Symbol::Class(bq)) if &bq != q => Some(bq),
                            _ => None,
                        })
                        .collect()
                })
                .unwrap_or_default();
            (q.clone(), bases)
        })
        .collect();
    let mut django: HashSet<String> = defs
        .iter()
        .filter(|(_, c, _)| {
            c.arguments
                .as_ref()
                .is_some_and(|a| a.args.iter().any(model_extractor::is_django_root))
        })
        .map(|(q, _, _)| q.clone())
        .collect();
    loop {
        let before = django.len();
        for (q, _, _) in &defs {
            if !django.contains(q) && base_classes[q].iter().any(|b| django.contains(b)) {
                django.insert(q.clone());
            }
        }
        if django.len() == before {
            break;
        }
    }

    // Own fields per model (line numbers resolved; choices resolved in the project).
    let mut own: HashMap<String, Vec<ModelField>> = HashMap::new();
    for (q, c, m) in &defs {
        if !django.contains(q) {
            continue;
        }
        let mut fields = model_extractor::extract_model_class(c, &m.relative_path, field_defaults);
        for f in &mut fields {
            f.source_line = m.lines.line(f.source_line);
            f.module = m.module.clone();
            if f.choices.is_none() {
                if let Some(expr) = &f.choices_expr {
                    f.choices = resolve_choices(index, &m.module, Some(q), expr, 0);
                }
            }
        }
        own.insert(q.clone(), fields);
    }

    fn with_inherited(
        q: &str,
        own: &HashMap<String, Vec<ModelField>>,
        bases: &HashMap<String, Vec<String>>,
        visiting: &mut HashSet<String>,
    ) -> Vec<ModelField> {
        if !visiting.insert(q.to_string()) || visiting.len() > 64 {
            return Vec::new();
        }
        let mut fields: Vec<ModelField> = Vec::new();
        for b in bases.get(q).into_iter().flatten() {
            if own.contains_key(b) {
                for f in with_inherited(b, own, bases, visiting) {
                    fields.retain(|e| e.field_name != f.field_name);
                    fields.push(f);
                }
            }
        }
        for f in own.get(q).into_iter().flatten() {
            match fields.iter().position(|e| e.field_name == f.field_name) {
                Some(i) => fields[i] = f.clone(),
                None => fields.push(f.clone()),
            }
        }
        visiting.remove(q);
        fields
    }

    let mut all = Vec::new();
    for (q, c, m) in &defs {
        if !django.contains(q) {
            continue;
        }
        let mut fields = with_inherited(q, &own, &base_classes, &mut HashSet::new());
        for f in &mut fields {
            // Inherited fields are reported under the child.
            f.model_name = c.name.to_string();
            f.module = m.module.clone();
        }
        if let Some(info) = index.classes.get_mut(q) {
            info.kind = ClassKind::Django;
            info.fields = fields.iter().map(|f| f.field_name.clone()).collect();
            info.field_nullable = fields
                .iter()
                .filter_map(|f| Some((f.field_name.clone(), f.null?)))
                .collect();
            info.field_facts = fields
                .iter()
                .map(|f| (f.field_name.clone(), model_extractor::requirement_facts(f)))
                .collect();
        }
        all.extend(fields);
    }
    all
}

/// The values a Django `choices=` expression allows: a literal list, a
/// module or class constant bound to one, or `SomeChoices.choices` for a
/// `TextChoices` / `IntegerChoices` class (its member values).
fn resolve_choices(
    index: &ProjectIndex,
    module: &str,
    class_q: Option<&str>,
    expr: &Expr,
    depth: usize,
) -> Option<Vec<String>> {
    if depth > 8 {
        return None;
    }
    if let Some(values) = model_extractor::literal_choices(expr) {
        return Some(values);
    }
    // A list of entries whose values name constants: `((OPEN, "Open"), ...)`.
    if let Expr::List(_) | Expr::Tuple(_) = expr {
        let items: &[Expr] = match expr {
            Expr::List(l) => &l.elts[..],
            Expr::Tuple(t) => &t.elts[..],
            _ => unreachable!(),
        };
        let mut out = Vec::new();
        for item in items {
            let entry: &[Expr] = match item {
                Expr::Tuple(t) => &t.elts[..],
                Expr::List(l) => &l.elts[..],
                other => std::slice::from_ref(other),
            };
            match entry {
                [value] | [value, _] => {
                    if let [_, group @ (Expr::List(_) | Expr::Tuple(_))] = entry {
                        if let Some(values) = resolve_choices(index, module, class_q, group, depth + 1) {
                            out.extend(values);
                            continue;
                        }
                    }
                    out.push(choice_value(index, module, class_q, value)?);
                }
                _ => return None,
            }
        }
        return (!out.is_empty()).then_some(out);
    }
    // A constant of the enclosing class body.
    if let (Expr::Name(n), Some(cq)) = (expr, class_q) {
        if let Some(q) = index.class_constant(cq, n.id.as_str()) {
            let value = index.constant(&q)?;
            return resolve_choices(index, module, class_q, value, depth + 1);
        }
    }
    // `Status.choices` on a choices class (module level, or nested in the
    // enclosing class body: `class Invoice: class Status(TextChoices): ...`).
    if let Expr::Attribute(a) = expr {
        if a.attr.as_str() == "choices" {
            let nested = match (a.value.as_ref(), class_q) {
                (Expr::Name(n), Some(cq)) => index.nested_class(cq, n.id.as_str()).map(Symbol::Class),
                _ => None,
            };
            if let Some(Symbol::Class(q)) = nested.or_else(|| index.resolve_expr(module, &a.value)) {
                let class = index.class(&q)?;
                if class.enum_kind == Some(EnumKind::DjangoChoices) {
                    return class.member_choices();
                }
            }
        }
    }
    match index.resolve_expr(module, expr)? {
        Symbol::Constant(q) => {
            let value = index.constant(&q)?;
            // A module constant is resolved in its own module.
            let owner = q.rsplit_once('.').map(|(m, _)| m).unwrap_or("");
            let owner = if index.modules.contains_key(owner) {
                owner.to_string()
            } else {
                module.to_string()
            };
            resolve_choices(index, &owner, None, value, depth + 1)
        }
        _ => None,
    }
}

/// One choice value: a literal, or a (class or module) constant bound to one.
fn choice_value(
    index: &ProjectIndex,
    module: &str,
    class_q: Option<&str>,
    expr: &Expr,
) -> Option<String> {
    if let Some(v) = resolve::literal_choice(expr) {
        return Some(v);
    }
    let q = match (expr, class_q) {
        (Expr::Name(n), Some(cq)) => index.class_constant(cq, n.id.as_str()),
        _ => None,
    }
    .or_else(|| match index.resolve_expr(module, expr)? {
        Symbol::Constant(q) => Some(q),
        _ => None,
    })?;
    // An enum member stands for its value.
    let enum_value = q
        .rsplit_once('.')
        .and_then(|(c, m)| index.class(c).and_then(|c| c.member_value(m)));
    resolve::literal_choice(enum_value.or_else(|| index.constant(&q))?)
}

/// pydantic fields typed with a project `Enum` / `TextChoices` class accept
/// its member values: a choices contract.
fn enum_field_choices(classes: &mut [DataClass], index: &ProjectIndex) {
    for dc in classes.iter_mut() {
        if dc.kind != dataclass_extractor::DataClassKind::Pydantic {
            continue;
        }
        for f in &mut dc.fields {
            if f.choices.is_some() {
                continue;
            }
            let Some(t) = f.type_name.clone() else { continue };
            let parts: Vec<String> = t.split('.').map(str::to_string).collect();
            let module = f
                .class_qualified
                .rsplit_once('.')
                .map(|(m, _)| m.to_string())
                .unwrap_or_default();
            if let Some(Symbol::Class(q)) = index.resolve_dotted(&module, &parts) {
                if let Some(values) = index.class(&q).and_then(|c| c.member_choices()) {
                    f.choices = Some(values);
                }
            }
        }
    }
}

/// Options of an extraction run.
#[derive(Debug, Clone, Default)]
pub struct ExtractOptions {
    /// Skip files with syntax errors (with a warning) instead of failing.
    pub allow_parse_errors: bool,
    /// Globs (`*`, `?`, `**`) over paths relative to the application root:
    /// a file is skipped when it, or one of its directories, matches one.
    pub exclude: Vec<String>,
}

/// Whether `path` (relative, `/`-separated) or one of its ancestor
/// directories matches `glob`: `**` matches any number of path segments,
/// `*` any characters within a segment, `?` one character.
pub fn glob_excludes(glob: &str, path: &str) -> bool {
    let pattern: Vec<&str> = glob
        .trim_start_matches("./")
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    (1..=segments.len()).any(|n| match_segments(&pattern, &segments[..n]))
}

fn match_segments(pattern: &[&str], path: &[&str]) -> bool {
    match pattern.split_first() {
        None => path.is_empty(),
        Some((&"**", rest)) => (0..=path.len()).any(|i| match_segments(rest, &path[i..])),
        Some((p, rest)) => match path.split_first() {
            Some((s, tail)) => match_segment(p.as_bytes(), s.as_bytes()) && match_segments(rest, tail),
            None => false,
        },
    }
}

fn match_segment(p: &[u8], s: &[u8]) -> bool {
    match p.split_first() {
        None => s.is_empty(),
        Some((b'*', rest)) => (0..=s.len()).any(|i| match_segment(rest, &s[i..])),
        Some((b'?', rest)) => !s.is_empty() && match_segment(rest, &s[1..]),
        Some((c, rest)) => s.first() == Some(c) && match_segment(rest, &s[1..]),
    }
}

/// Run the full extraction pipeline on a Python project directory (or a single .py file).
///
/// If `output_db` is `Some(path)`, the SQLite database is written there.
/// If `None`, a default temp path is used.
pub fn extract(
    app_path: &Path,
    overrides_path: Option<&Path>,
    django_version: &str,
    output_db: Option<&Path>,
) -> Result<std::path::PathBuf> {
    extract_with(
        app_path,
        overrides_path,
        django_version,
        output_db,
        &ExtractOptions::default(),
    )
}

/// `extract` with options. A file with syntax errors fails the run unless
/// `options.allow_parse_errors`, in which case the file is skipped.
pub fn extract_with(
    app_path: &Path,
    overrides_path: Option<&Path>,
    django_version: &str,
    output_db: Option<&Path>,
    options: &ExtractOptions,
) -> Result<std::path::PathBuf> {
    if !app_path.exists() {
        anyhow::bail!("Application path does not exist: {}", app_path.display());
    }

    // Determine database path
    let db_path = match output_db {
        Some(path) => {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            path.to_path_buf()
        }
        None => {
            let db_dir = std::env::temp_dir().join("crosscheck");
            std::fs::create_dir_all(&db_dir)?;
            db_dir.join("contracts.sqlite")
        }
    };

    // Load defaults
    let field_defaults = defaults::load_defaults(django_version)?;

    // Parse all Python files
    let py_files = python_files(app_path, &options.exclude)?;
    let mut modules = Vec::new();
    let mut parse_errors = Vec::new();
    for py_file in &py_files {
        let bytes = std::fs::read(py_file)?;
        let source = String::from_utf8_lossy(&bytes).into_owned();
        let mut relative_path = py_file
            .strip_prefix(app_path)
            .unwrap_or(py_file)
            .to_string_lossy()
            .to_string();
        if relative_path.is_empty() {
            // A single file was given: name the module after the file.
            relative_path = py_file
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
        }
        let mut module = SourceModule::parse(&relative_path, source);
        if let Some(err) = module.parse_error.take() {
            parse_errors.push(format!("{relative_path}: {err}"));
            if options.allow_parse_errors {
                // Skip the file: a recovered syntax tree is not trusted.
                module.stmts.clear();
            }
        }
        modules.push(module);
    }
    if !parse_errors.is_empty() {
        if !options.allow_parse_errors {
            anyhow::bail!(
                "{} file(s) have syntax errors; the run would be incomplete \
                 (pass --allow-parse-errors to skip them):\n  {}",
                parse_errors.len(),
                parse_errors.join("\n  ")
            );
        }
        for e in &parse_errors {
            eprintln!("Warning: skipped (syntax error) {e}");
        }
    }

    // Remove existing database
    if db_path.exists() {
        std::fs::remove_file(&db_path)?;
    }
    let db = ContractDb::create(&db_path)?;

    let project = Project::build(modules, &field_defaults);
    let names = resolve::display_names(&project.node_names());

    // Nodes: model fields, data class fields, functions
    let mut model_node_ids =
        model_extractor::write_model_fields(&db, &project.model_fields, &names)?;
    let data_class_ids =
        dataclass_extractor::write_data_classes(&db, &project.index, &project.data_classes, &names)?;
    for (q, id) in data_class_ids {
        model_node_ids.entry(q).or_insert(id);
    }
    let (func_node_ids, func_rows) = write_functions(&db, &project, &names)?;
    for (q, id) in write_return_nodes(&db, &project, &names)? {
        model_node_ids.entry(q).or_insert(id);
    }

    // Edges, and the call-site nodes they use
    let discovered = edge_discovery::discover(&project);
    let site_ids = write_call_sites(
        &db,
        &discovered.call_sites,
        &func_rows,
        &names,
    )?;
    let ids = NodeIds {
        functions: &func_node_ids,
        models: &model_node_ids,
        sites: &site_ids,
        names: &names,
    };
    let edges = dedupe(discovered.edges);
    let (mut edge_count, dropped) = write_edges(&db, &edges, &ids)?;
    // Discovered edges name nodes by the qualified names they were written
    // under, so a drop means the discoverer and the writers disagree: an
    // edge, and the check on it, silently missing.
    if dropped > 0 {
        eprintln!(
            "Warning: {dropped} discovered edge(s) dropped: an endpoint is not an extracted node"
        );
    }

    // Load and write manual overrides
    if let Some(overrides_path) = overrides_path {
        if overrides_path.exists() {
            let override_edges = edge_discovery::load_overrides(overrides_path)?;
            let (written, dropped) = write_edges(&db, &override_edges, &ids)?;
            edge_count += written;
            if dropped > 0 {
                eprintln!(
                    "Warning: {dropped} manual override edge(s) dropped: an endpoint matches no extracted node"
                );
            }
        }
    }
    db.finish()?;

    let data_class_field_count: usize = project.data_classes.iter().map(|c| c.fields.len()).sum();
    let function_count = project
        .index
        .functions
        .iter()
        .filter(|f| !f.name.starts_with("<module "))
        .count();
    eprintln!(
        "Extracted {} model fields, {} data class fields, {} functions ({} module-level code blocks, {} call sites), {} edges to {}",
        project.model_fields.len(),
        data_class_field_count,
        function_count,
        project.index.functions.len() - function_count,
        site_ids.len(),
        edge_count,
        db_path.display()
    );

    Ok(db_path)
}

/// Write function nodes with their pre- and postconditions.
/// Returns qualified name → node ID, and qualified name → the rows written
/// (for copying onto call-site nodes).
#[allow(clippy::type_complexity)]
fn write_functions(
    db: &ContractDb,
    project: &Project,
    names: &HashMap<String, String>,
) -> Result<(HashMap<String, i64>, HashMap<String, Vec<ContractRecord>>)> {
    let mut ids = HashMap::new();
    let mut all_rows = HashMap::new();
    for func in &project.index.functions {
        let node_id = db.insert_node(&NodeRecord {
            name: names
                .get(&func.qualified_name)
                .cloned()
                .unwrap_or_else(|| func.name.clone()),
            qualified_name: Some(func.qualified_name.clone()),
            kind: NodeKind::Function,
            source_file: func.source_file.clone(),
            source_line: func.source_line,
            is_call_site: false,
        })?;
        ids.insert(func.qualified_name.clone(), node_id);

        let doc = project
            .docstrings
            .get(&func.qualified_name)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let summary = project
            .summaries
            .get(&func.qualified_name)
            .cloned()
            .unwrap_or_default();
        let rows: Vec<ContractRecord> = function_extractor::precondition_rows(func, doc, node_id)
            .into_iter()
            .chain(function_extractor::postcondition_rows(
                &project.index, func, &summary, doc, node_id,
            ))
            .collect();
        for row in &rows {
            db.insert_contract(row)?;
        }
        all_rows.insert(func.qualified_name.clone(), rows);
    }
    Ok((ids, all_rows))
}

/// Write the return contract node `f.<return>` of every function whose
/// return annotation excludes None (kind `model`, located at the
/// annotation): preconditions non-null and, for `Decimal` / `str` / `bool`,
/// the type. Every `return` of `f` is an edge into it, so callers relying
/// on the annotation is assume-guarantee. Returns qualified name → node ID.
fn write_return_nodes(
    db: &ContractDb,
    project: &Project,
    names: &HashMap<String, String>,
) -> Result<HashMap<String, i64>> {
    use crate::db::{ConstraintType, ContractRole, VerificationLevel};
    let mut ids = HashMap::new();
    for func in project.index.functions.iter().filter(|f| f.has_return_contract()) {
        let q = func.return_node_name();
        let node_id = db.insert_node(&NodeRecord {
            name: names
                .get(&q)
                .cloned()
                .unwrap_or_else(|| format!("{}.<return>", func.name)),
            qualified_name: Some(q.clone()),
            kind: NodeKind::Model,
            source_file: func.source_file.clone(),
            source_line: func.return_line,
            is_call_site: false,
        })?;
        let base = |kind: ConstraintType| {
            ContractRecord::new(
                node_id,
                kind,
                ContractRole::Precondition,
                VerificationLevel::Extracted,
                &func.source_file,
                func.return_line,
            )
        };
        db.insert_contract(&ContractRecord {
            param_nullable: Some(0),
            ..base(ConstraintType::Nullability)
        })?;
        // A project class that shadows a value type keeps its qualified name.
        let type_name = func.return_type.as_deref().filter(|t| !t.contains('[')).and_then(|t| {
            if project.index.is_contract_type(t) && t.contains('.') {
                return Some(t);
            }
            let last = t.rsplit('.').next().unwrap_or(t);
            function_extractor::RETURN_TYPE_CONTRACTS.contains(&last).then_some(last)
        });
        if let Some(t) = type_name {
            db.insert_contract(&ContractRecord {
                param_type_name: Some(t.to_string()),
                ..base(ConstraintType::Type)
            })?;
        }
        ids.insert(q, node_id);
    }
    Ok(ids)
}

/// Write one node per consumed call site (kind `function`, the callee's
/// display name, qualified `callee@file:line`, located at the call) with
/// copies of the callee's contract rows. Returns site → node ID.
fn write_call_sites(
    db: &ContractDb,
    sites: &[CallSite],
    func_rows: &HashMap<String, Vec<ContractRecord>>,
    names: &HashMap<String, String>,
) -> Result<HashMap<SiteKey, i64>> {
    let mut ids = HashMap::new();
    for site in sites {
        let Some(rows) = func_rows.get(&site.callee) else {
            continue;
        };
        let node_id = db.insert_node(&NodeRecord {
            name: names
                .get(&site.callee)
                .cloned()
                .unwrap_or_else(|| site.callee.clone()),
            qualified_name: Some(format!("{}@{}:{}", site.callee, site.key.file, site.line)),
            kind: NodeKind::Function,
            source_file: site.key.file.clone(),
            source_line: site.line,
            is_call_site: true,
        })?;
        for row in rows {
            db.insert_contract(&ContractRecord {
                node_id,
                ..row.clone()
            })?;
        }
        ids.insert(site.key.clone(), node_id);
    }
    Ok(ids)
}

/// Drop exact repeats of edges that carry no override rows, and repeated
/// `calls` edges between the same two functions (they are not checked).
fn dedupe(edges: Vec<DiscoveredEdge>) -> Vec<DiscoveredEdge> {
    let mut seen = HashSet::new();
    let mut calls = HashSet::new();
    edges
        .into_iter()
        .filter(|e| {
            if e.relationship == "calls" {
                return calls.insert((e.source_function.clone(), e.target_name.clone()));
            }
            e.override_rows.is_some()
                || seen.insert((
                    e.source_function.clone(),
                    e.source_site.clone(),
                    e.target_name.clone(),
                    e.target_field.clone(),
                    e.relationship.clone(),
                    e.target_param.clone(),
                    e.target_site.clone(),
                    e.site.clone(),
                ))
        })
        .collect()
}

/// Node IDs by what edges name them with.
struct NodeIds<'a> {
    functions: &'a HashMap<String, i64>,
    models: &'a HashMap<String, i64>,
    sites: &'a HashMap<SiteKey, i64>,
    names: &'a HashMap<String, String>,
}

/// Write edges (and each edge's override rows, after the edge itself).
/// Edges whose endpoints are not extracted nodes are dropped. Returns
/// (edges written, edges dropped for an unknown endpoint).
fn write_edges(
    db: &ContractDb,
    edges: &[DiscoveredEdge],
    ids: &NodeIds,
) -> Result<(usize, usize)> {
    let mut written = 0;
    let mut dropped = 0;
    for edge in edges {
        // Discovered edges carry qualified names; only manual overrides are
        // matched loosely (display name or dotted suffix).
        let lookup = |map: &HashMap<String, i64>, name: &str| {
            if edge.discovery == "manual" {
                lookup(map, ids.names, name)
            } else {
                map.get(name).copied()
            }
        };
        let source_id = match &edge.source_site {
            Some(key) => ids.sites.get(key).copied(),
            None => lookup(ids.functions, &edge.source_function),
        };
        let target_id = match (&edge.target_site, &edge.target_field) {
            (Some(key), _) => ids.sites.get(key).copied(),
            (None, Some(field)) => lookup(ids.models, &format!("{}.{}", edge.target_name, field)),
            (None, None) => lookup(ids.models, &edge.target_name)
                .or_else(|| lookup(ids.functions, &edge.target_name)),
        };
        let (Some(src_id), Some(tgt_id)) = (source_id, target_id) else {
            dropped += 1;
            continue;
        };
        let relationship = match edge.relationship.as_str() {
            "calls" => Relationship::Calls,
            "writes_to" => Relationship::WritesTo,
            "flows_to" => Relationship::FlowsTo,
            _ => continue,
        };
        let discovery = match edge.discovery.as_str() {
            "ast_pattern" => Discovery::AstPattern,
            "manual" => Discovery::Manual,
            "type_inference" => Discovery::TypeInference,
            _ => continue,
        };

        let edge_id = db.insert_edge(&EdgeRecord {
            source_node_id: src_id,
            target_node_id: tgt_id,
            relationship,
            discovery,
            target_param: edge.target_param.clone(),
            source_override: edge.override_rows.is_some(),
            site_file: edge.site.as_ref().map(|s| s.0.clone()),
            site_line: edge.site.as_ref().map(|s| s.1),
        })?;
        written += 1;
        for row in edge.override_rows.iter().flatten() {
            let mut row = row.clone();
            row.node_id = src_id;
            row.edge_id = Some(edge_id);
            db.insert_contract(&row)?;
        }
    }
    Ok((written, dropped))
}

/// A node ID by qualified name, else by display name, else by a unique
/// dotted-suffix match (for manual overrides naming `pkg.module.func`
/// or a short name).
fn lookup(ids: &HashMap<String, i64>, names: &HashMap<String, String>, name: &str) -> Option<i64> {
    if let Some(&id) = ids.get(name) {
        return Some(id);
    }
    let by_display: Vec<i64> = ids
        .iter()
        .filter(|(q, _)| names.get(*q).map(String::as_str) == Some(name))
        .map(|(_, &id)| id)
        .collect();
    if let [id] = by_display.as_slice() {
        return Some(*id);
    }
    let suffix = |a: &str, b: &str| {
        a.len() > b.len() && a.ends_with(b) && a[..a.len() - b.len()].ends_with('.')
    };
    let by_suffix: Vec<i64> = ids
        .iter()
        .filter(|(q, _)| suffix(q, name) || suffix(name, q))
        .map(|(_, &id)| id)
        .collect();
    match by_suffix.as_slice() {
        [id] => Some(*id),
        _ => None,
    }
}

/// The .py files a run under `app_path` analyses, sorted, after `--exclude`.
pub fn python_files(app_path: &Path, exclude: &[String]) -> Result<Vec<std::path::PathBuf>> {
    let mut py_files = find_python_files(app_path)?;
    py_files.sort();
    py_files.retain(|f| {
        let rel = f
            .strip_prefix(app_path)
            .unwrap_or(f)
            .to_string_lossy()
            .replace('\\', "/");
        !exclude.iter().any(|g| glob_excludes(g, &rel))
    });
    Ok(py_files)
}

/// Find all .py files in a directory recursively.
fn find_python_files(dir: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    if dir.is_file() && dir.extension().is_some_and(|ext| ext == "py") {
        files.push(dir.to_path_buf());
        return Ok(files);
    }
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                // Skip __pycache__, .git, etc.
                let dir_name = path.file_name().unwrap_or_default().to_string_lossy();
                if !dir_name.starts_with('.')
                    && dir_name != "__pycache__"
                    && dir_name != "node_modules"
                {
                    files.extend(find_python_files(&path)?);
                }
            } else if path.extension().is_some_and(|ext| ext == "py") {
                files.push(path);
            }
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use crate::bounds::mu;

    fn project(files: &[(&str, &str)]) -> Project {
        Project::from_sources(
            files
                .iter()
                .map(|(p, s)| (p.to_string(), s.to_string()))
                .collect(),
        )
    }

    /// `Model.field (decimal places, max length)` per extracted model field, sorted.
    fn model_fields(p: &Project) -> Vec<String> {
        let mut v: Vec<String> = p
            .model_fields
            .iter()
            .map(|f| {
                format!(
                    "{} dp={:?} len={:?} line={}",
                    model_extractor::field_qualified(f),
                    f.decimal_places,
                    f.max_length,
                    f.source_line
                )
            })
            .collect();
        v.sort();
        v
    }

    /// Round 3 D3: models inheriting from project-local bases.
    #[test]
    fn test_django_inheritance() {
        let p = project(&[
            (
                "common.py",
                "from django.db import models\n\
                 class TimeStamped(models.Model):\n    amount = models.DecimalField(max_digits=10, decimal_places=2)\n    class Meta:\n        abstract = True\n\
                 class Place(models.Model):\n    name = models.CharField(max_length=5)\n",
            ),
            (
                "models.py",
                "from django.db import models\nfrom common import Place, TimeStamped\n\
                 class Payment(TimeStamped):\n    fee = models.DecimalField(max_digits=10, decimal_places=2)\n\
                 class Restaurant(Place):\n    seats = models.IntegerField()\n\
                 class Precise(TimeStamped):\n    amount = models.DecimalField(max_digits=12, decimal_places=4)\n\
                 class Grandchild(Payment):\n    pass\n\
                 class NotAModel(Helper):\n    x = models.IntegerField()\n",
            ),
        ]);
        assert_eq!(
            model_fields(&p),
            [
                "common.Place.name dp=None len=Some(5) line=7",
                "common.TimeStamped.amount dp=Some(2) len=None line=3",
                "models.Grandchild.amount dp=Some(2) len=None line=3",
                "models.Grandchild.fee dp=Some(2) len=None line=4",
                "models.Payment.amount dp=Some(2) len=None line=3",
                "models.Payment.fee dp=Some(2) len=None line=4",
                "models.Precise.amount dp=Some(4) len=None line=8",
                "models.Restaurant.name dp=None len=Some(5) line=7",
                "models.Restaurant.seats dp=None len=None line=6",
            ]
        );
        // Inherited fields keep the parent's file.
        let inherited = p
            .model_fields
            .iter()
            .find(|f| f.model_name == "Payment" && f.field_name == "amount")
            .unwrap();
        assert_eq!(inherited.source_file, "common.py");
        let class = p.index.class("models.Grandchild").unwrap();
        assert_eq!(class.kind, ClassKind::Django);
        assert_eq!(class.fields, ["amount", "fee"]);
    }

    #[test]
    fn test_django_inheritance_cycle_terminates() {
        let p = project(&[(
            "models.py",
            "from django.db import models\nclass A(B):\n    x = models.IntegerField()\nclass B(A):\n    y = models.IntegerField()\nclass C(models.Model, C):\n    z = models.IntegerField()\n",
        )]);
        assert_eq!(model_fields(&p), ["models.C.z dp=None len=None line=7"]);
    }

    /// Round 3 D5: `choices=` forms resolved with the project.
    #[test]
    fn test_choices_resolution() {
        let p = project(&[
            (
                "consts.py",
                "COLOURS = [('red', _('Red')), ('blue', _('Blue'))]\n",
            ),
            (
                "models.py",
                "from django.db import models\nfrom consts import COLOURS\nimport consts\n\
                 class Status(models.TextChoices):\n    ACTIVE = 'active', _('Active')\n    EXPIRED = 'expired'\n\
                 class Level(models.IntegerChoices):\n    LOW = 1, 'Low'\n    HIGH = 2, 'High'\n\
                 SIZES = (('s', 'S'), ('l', 'L'))\n\
                 class Order(models.Model):\n    OPEN = 'open'\n    FROZEN = 'frozen'\n    STATES = ((OPEN, 'Open'), (FROZEN, 'Frozen'))\n\
                 \x20   status = models.CharField(max_length=9, choices=Status.choices)\n\
                 \x20   level = models.IntegerField(choices=Level.choices)\n\
                 \x20   size = models.CharField(max_length=1, choices=SIZES)\n\
                 \x20   colour = models.CharField(max_length=5, choices=COLOURS)\n\
                 \x20   colour2 = models.CharField(max_length=5, choices=consts.COLOURS)\n\
                 \x20   state = models.CharField(max_length=6, choices=STATES)\n\
                 \x20   dynamic = models.CharField(max_length=6, choices=get_choices())\n",
            ),
        ]);
        let choices = |name: &str| {
            p.model_fields
                .iter()
                .find(|f| f.field_name == name)
                .unwrap()
                .choices
                .clone()
        };
        let v = |xs: &[&str]| Some(xs.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(choices("status"), v(&["active", "expired"]));
        assert_eq!(choices("level"), v(&["1", "2"]));
        assert_eq!(choices("size"), v(&["s", "l"]));
        assert_eq!(choices("colour"), v(&["red", "blue"]));
        assert_eq!(choices("colour2"), v(&["red", "blue"]));
        assert_eq!(choices("state"), v(&["open", "frozen"]));
        assert_eq!(choices("dynamic"), None);
    }

    /// Round 3 D4 / D5 / F4 on data classes.
    #[test]
    fn test_data_class_bounds_choices_and_strictness() {
        let p = project(&[(
            "records.py",
            "from enum import Enum\n\
             class Colour(str, Enum):\n    RED = 'red'\n    BLUE = 'blue'\n\
             class R(BaseModel):\n    ratio: float = Field(ge=0.0, le=0.5)\n    amt: Decimal = Field(le=Decimal('10'))\n    amt2: condecimal(ge=Decimal('0'))\n    small: float = Field(gt=0, lt=1)\n    kind: Literal['a', 'b']\n    colour: Colour\n    n: int = Field(strict=True)\n\
             class S(BaseModel):\n    model_config = ConfigDict(strict=True)\n    x: Decimal\n\
             class T(S):\n    y: int\n",
        )]);
        let dc = |name: &str| p.data_classes.iter().find(|c| c.name == name).unwrap();
        let f = |c: &str, n: &str| dc(c).fields.iter().find(|f| f.field_name == n).unwrap().clone();
        assert_eq!((f("R", "ratio").min_value, f("R", "ratio").max_value), (mu(0), mu(500_000)));
        assert_eq!(f("R", "amt").max_value, mu(10_000_000));
        assert_eq!(f("R", "amt2").min_value, mu(0));
        // Strict bounds on a float: one millionth inside (stricter than required).
        assert_eq!((f("R", "small").min_value, f("R", "small").max_value), (mu(1), mu(999_999)));
        assert_eq!(f("R", "kind").choices, Some(vec!["a".to_string(), "b".to_string()]));
        assert_eq!(f("R", "colour").choices, Some(vec!["red".to_string(), "blue".to_string()]));
        let r = dc("R");
        assert!(r.lax_numeric(&f("R", "ratio")));
        assert!(!r.lax_numeric(&f("R", "n")), "Field(strict=True)");
        assert!(!dc("S").lax_numeric(&f("S", "x")), "model_config strict");
        assert!(!dc("T").lax_numeric(&f("T", "y")), "strictness is inherited");
    }

    #[test]
    fn test_exclude_globs() {
        assert!(glob_excludes("**/tests/**", "netbox/tests/test_api.py"));
        assert!(glob_excludes("**/tests/**", "tests/test_api.py"));
        assert!(glob_excludes("**/tests", "a/b/tests/x.py"), "a matching directory excludes its files");
        assert!(!glob_excludes("**/tests/**", "netbox/testsuite/x.py"));
        assert!(glob_excludes("**/test_*.py", "app/test_models.py"));
        assert!(!glob_excludes("test_*.py", "app/test_models.py"), "no `**`: anchored at the root");
        assert!(glob_excludes("*/migrations/*", "app/migrations/0001_initial.py"));
        assert!(glob_excludes("app/m?dels.py", "app/models.py"));
        assert!(!glob_excludes("app/*.py", "app/sub/models.py"));
    }

    #[test]
    fn test_parse_errors_are_reported() {
        let m = SourceModule::parse("bad.py", "def f(:\n    pass\n".to_string());
        assert!(m.parse_error.as_deref().is_some_and(|e| e.starts_with("line 1:")), "{:?}", m.parse_error);
        // Python 3.14 syntax (PEP 758) parses.
        let m = SourceModule::parse(
            "new.py",
            "def f(x):\n    try:\n        return int(x)\n    except ValueError, TypeError:\n        return 0\n"
                .to_string(),
        );
        assert_eq!(m.parse_error, None);
    }

    /// Round 5 N5 / N6: classes (and functions) in module-level `if` / `try`
    /// / `with` blocks, and classes nested in class bodies.
    #[test]
    fn test_conditional_and_nested_classes() {
        let p = project(&[(
            "models.py",
            "from django.db import models\n\
             if not registered('Price'):\n    class Price(models.Model):\n        amount = models.DecimalField(max_digits=5, decimal_places=2)\n\
             try:\n    class Fee(models.Model):\n        x = models.IntegerField()\nexcept ImportError:\n    pass\n\
             with ctx():\n    def helper():\n        return 1\n\
             class Invoice(models.Model):\n    class Status(models.TextChoices):\n        DRAFT = 'draft', 'Draft'\n        PAID = 'paid', 'Paid'\n\
             \x20   status = models.CharField(max_length=5, choices=Status.choices)\n\
             class Child(Invoice):\n    other = models.CharField(max_length=5, choices=Status.choices)\n",
        )]);
        let fields = model_fields(&p);
        assert!(fields.iter().any(|f| f.starts_with("models.Price.amount dp=Some(2)")), "{fields:?}");
        assert!(fields.iter().any(|f| f.starts_with("models.Fee.x")), "{fields:?}");
        assert!(p.index.function("models.helper").is_some());
        let status = p.model_fields.iter().find(|f| f.model_name == "Invoice" && f.field_name == "status").unwrap();
        assert_eq!(status.choices, Some(vec!["draft".to_string(), "paid".to_string()]));
        assert!(p.index.class("models.Invoice.Status").is_some());
        // Inside a subclass body the bare name does not resolve in Python either,
        // but through the base's nested class it does here.
        let other = p.model_fields.iter().find(|f| f.field_name == "other").unwrap();
        assert_eq!(other.choices, Some(vec!["draft".to_string(), "paid".to_string()]));
    }

    /// Round 6: `Annotated` aliases (local, imported, PEP 695), and
    /// `BaseSettings` / `SQLModel` classes as pydantic.
    #[test]
    fn test_annotated_aliases_and_pydantic_bases() {
        let p = project(&[
            ("types_.py", "from typing import Annotated\nfrom pydantic import Field\nPercent = Annotated[int, Field(ge=0, le=100)]\n"),
            (
                "schemas.py",
                "from typing import Annotated\nfrom pydantic import BaseModel, Field\nfrom pydantic_settings import BaseSettings\nfrom sqlmodel import SQLModel\nfrom types_ import Percent\n\
                 type Code = Annotated[str, Field(max_length=4)]\n\
                 class B(BaseModel):\n    level: Percent\n    code: Code\n\
                 class S(BaseSettings):\n    workers: int = Field(ge=1)\n\
                 class H(SQLModel, table=True):\n    name: str = Field(max_length=8)\n",
            ),
        ]);
        let f = |c: &str, n: &str| {
            p.data_classes
                .iter()
                .find(|d| d.name == c)
                .unwrap_or_else(|| panic!("{c}"))
                .fields
                .iter()
                .find(|x| x.field_name == n)
                .unwrap()
                .clone()
        };
        assert_eq!((f("B", "level").min_value, f("B", "level").max_value), (mu(0), mu(100_000_000)));
        assert_eq!(f("B", "code").max_length, Some(4));
        assert_eq!(f("S", "workers").min_value, mu(1_000_000));
        assert_eq!(f("H", "name").max_length, Some(8));
    }

    /// Annotations naming a module-level union alias or a type variable.
    #[test]
    fn test_annotation_aliases_and_type_vars() {
        let p = project(&[(
            "code.py",
            "from typing import Optional, TypeVar\nfrom dataclasses import dataclass\n\
             Requestor = App | User | None\nMaybe = Optional[int]\nN = TypeVar('N')\n\
             def f(r: Requestor, m: Maybe, n: N, s: str): pass\n\
             @dataclass\nclass Box[T]:\n    item: T\n    other: N\n    who: Requestor\n    name: str\n",
        )]);
        let f = p.index.function("code.f").unwrap();
        let null: Vec<Option<bool>> = f.params.iter().map(|p| p.nullable).collect();
        assert_eq!(null, [Some(true), Some(true), None, Some(false)]);
        assert_eq!(f.params[1].type_name.as_deref(), Some("int"));
        let b = p.data_classes.iter().find(|c| c.name == "Box").unwrap();
        let null: Vec<Option<bool>> = b.fields.iter().map(|f| f.nullable).collect();
        assert_eq!(null, [None, None, Some(true), Some(false)]);
    }
}
