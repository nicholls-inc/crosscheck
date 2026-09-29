//! Syntactic TypeScript types: `TypeIndex` (every alias, interface and enum
//! in the project, with imports resolved through `ImportMap` and
//! `tsconfig.json`) and `Shape`, what a type expression requires of a value
//! as far as syntax can tell. Anything syntax cannot settle is `Unknown`.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use oxc_ast::ast::*;
use oxc_span::GetSpan;

use super::ParsedModule;
use crate::value_analysis::ValueFacts;

/// Index of a module in the parsed set.
pub type FileId = usize;

/// `compilerOptions.baseUrl` and `paths` of the nearest `tsconfig.json`,
/// as absolute path rules (`extends` is not followed).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TsConfig {
    /// Non-relative specifiers also resolve against this directory.
    base_url: Option<PathBuf>,
    /// `paths` entries: (pattern with at most one `*`, targets relative to `base`).
    rules: Vec<(String, Vec<String>)>,
    /// The directory `paths` targets resolve against (`baseUrl` if set).
    base: PathBuf,
}

impl TsConfig {
    /// The nearest `tsconfig.json` in `app_root` or one of its ancestors;
    /// none, or an unreadable one (with a warning), gives empty rules.
    pub fn load(app_root: &Path) -> TsConfig {
        let path = std::iter::once(app_root)
            .chain(app_root.ancestors().skip(1))
            .map(|d| d.join("tsconfig.json"))
            .find(|p| p.is_file());
        let Some(path) = path else {
            return TsConfig::default();
        };
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let dir = path.parent().unwrap_or(app_root);
        match TsConfig::parse(&text, dir) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Warning: {}: {e}; import paths not resolved", path.display());
                TsConfig::default()
            }
        }
    }

    /// Parse tsconfig text (comments and trailing commas allowed); `dir` is
    /// the directory holding it.
    pub fn parse(text: &str, dir: &Path) -> anyhow::Result<TsConfig> {
        let json: serde_json::Value = serde_json::from_str(&strip_jsonc(text))?;
        let options = &json["compilerOptions"];
        let base_url = options["baseUrl"]
            .as_str()
            .map(|b| normalize(&dir.join(b)));
        let base = base_url.clone().unwrap_or_else(|| normalize(dir));
        let mut rules = Vec::new();
        if let Some(paths) = options["paths"].as_object() {
            for (pattern, targets) in paths {
                let targets: Vec<String> = targets
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|t| t.as_str().map(str::to_string))
                    .collect();
                rules.push((pattern.clone(), targets));
            }
        }
        // Longest pattern prefix wins, as in tsc.
        rules.sort_by_key(|(p, _)| std::cmp::Reverse(p.split('*').next().unwrap_or("").len()));
        Ok(TsConfig { base_url, rules, base })
    }

    /// Absolute paths (without extension) a non-relative specifier may name,
    /// in priority order.
    pub fn candidates(&self, specifier: &str) -> Vec<PathBuf> {
        let mut out = Vec::new();
        for (pattern, targets) in &self.rules {
            let captured = match pattern.split_once('*') {
                Some((prefix, suffix)) => {
                    if specifier.len() >= prefix.len() + suffix.len()
                        && specifier.starts_with(prefix)
                        && specifier.ends_with(suffix)
                    {
                        Some(&specifier[prefix.len()..specifier.len() - suffix.len()])
                    } else {
                        None
                    }
                }
                None => (pattern == specifier).then_some(""),
            };
            if let Some(captured) = captured {
                for t in targets {
                    out.push(normalize(&self.base.join(t.replace('*', captured))));
                }
            }
        }
        if let Some(base) = &self.base_url {
            out.push(normalize(&base.join(specifier)));
        }
        out
    }
}

/// JSON with `//` and `/* */` comments and trailing commas removed.
pub fn strip_jsonc(text: &str) -> String {
    let b = text.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'"' => {
                let start = i;
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    i += if b[i] == b'\\' { 2 } else { 1 };
                }
                i = (i + 1).min(b.len());
                out.extend_from_slice(&b[start..i]);
            }
            b'/' if b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(b.len());
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    let mut result: Vec<u8> = Vec::with_capacity(out.len());
    let mut i = 0;
    let mut in_string = false;
    while i < out.len() {
        let c = out[i];
        if in_string {
            result.push(c);
            if c == b'\\' && i + 1 < out.len() {
                result.push(out[i + 1]);
                i += 1;
            } else if c == b'"' {
                in_string = false;
            }
        } else if c == b'"' {
            in_string = true;
            result.push(c);
        } else if c == b',' {
            let mut j = i + 1;
            while j < out.len() && out[j].is_ascii_whitespace() {
                j += 1;
            }
            if !(j < out.len() && (out[j] == b'}' || out[j] == b']')) {
                result.push(c);
            }
        } else {
            result.push(c);
        }
        i += 1;
    }
    String::from_utf8_lossy(&result).into_owned()
}

/// Lexical normalisation: `.` and `..` components resolved, no filesystem access.
pub fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            c => out.push(c.as_os_str()),
        }
    }
    out
}

/// What a type expression requires of a value, as far as syntax can tell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shape {
    /// `string`, `number`, `boolean`, `bigint`, a boolean literal: non-null, no choices.
    Primitive,
    /// A union of string / number literals (or an enum): the choices.
    Literals(Vec<String>),
    /// `null` or `undefined` was a union member.
    Nullable(Box<Shape>),
    /// An interface or object type: one slot per property.
    Object(Vec<Field>),
    /// A generic, conditional, mapped, indexed or `keyof` type, an intersection,
    /// an unresolved import, a cycle: never guessed.
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub optional: bool,
    pub shape: Shape,
    /// Where the property is declared.
    pub file: FileId,
    pub line: u32,
}

/// The precondition rows of a slot. `None` means "not determinable".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Requirement {
    /// `Some(false)`: rejects null / undefined. `Some(true)`: accepts them.
    pub nullable: Option<bool>,
    /// Every member of a literal union.
    pub choices: Option<Vec<String>>,
}

impl Requirement {
    /// Whether some value fails this requirement.
    pub fn can_reject(&self) -> bool {
        self.nullable == Some(false) || self.choices.is_some()
    }
}

impl Shape {
    /// The union of `members`, plus `null` / `undefined` when `nullable`.
    /// Literals join; one member stands alone; an `Unknown` member, or two
    /// members that are not all literals with an object among them, give
    /// `Unknown`; primitives and literals mixed give `Primitive`. An
    /// explicit null stays visible past an `Unknown`.
    pub fn union(members: Vec<Shape>, nullable: bool) -> Shape {
        let mut nullable = nullable;
        let mut flat = Vec::new();
        for m in members {
            match m {
                Shape::Nullable(inner) => {
                    nullable = true;
                    flat.push(*inner);
                }
                m => flat.push(m),
            }
        }
        let core = if flat.is_empty() {
            Shape::Unknown
        } else if flat.iter().all(|s| matches!(s, Shape::Literals(_))) {
            let mut all: Vec<String> = Vec::new();
            for s in flat {
                if let Shape::Literals(v) = s {
                    for x in v {
                        if !all.contains(&x) {
                            all.push(x);
                        }
                    }
                }
            }
            Shape::Literals(all)
        } else if flat.iter().any(|s| matches!(s, Shape::Unknown)) {
            Shape::Unknown
        } else if flat.len() == 1 {
            flat.pop().unwrap()
        } else if flat.iter().any(|s| matches!(s, Shape::Object(_))) {
            Shape::Unknown
        } else {
            Shape::Primitive
        };
        if nullable {
            Shape::Nullable(Box::new(core))
        } else {
            core
        }
    }

    fn choices(&self) -> Option<Vec<String>> {
        match self {
            Shape::Literals(v) => Some(v.clone()),
            _ => None,
        }
    }

    /// The slot rows for a value of this shape (`optional`: a `?:` property).
    pub fn requirement(&self, optional: bool) -> Requirement {
        let (nullable, choices) = match self {
            Shape::Unknown => (None, None),
            Shape::Nullable(inner) => (Some(true), inner.choices()),
            Shape::Literals(v) => (Some(false), Some(v.clone())),
            Shape::Primitive | Shape::Object(_) => (Some(false), None),
        };
        Requirement {
            nullable: if optional { Some(true) } else { nullable },
            choices,
        }
    }

    /// What a value known to have this type guarantees.
    pub fn facts(&self) -> ValueFacts {
        let r = self.requirement(false);
        ValueFacts {
            nullable: r.nullable,
            choices: r.choices,
            ..Default::default()
        }
    }
}

enum TypeDecl<'a> {
    Alias(&'a TSType<'a>),
    Interface(&'a TSInterfaceDeclaration<'a>),
    Enum(&'a TSEnumDeclaration<'a>),
}

impl TypeDecl<'_> {
    fn span(&self) -> Span {
        match self {
            TypeDecl::Alias(t) => t.span(),
            TypeDecl::Interface(i) => i.span,
            TypeDecl::Enum(e) => e.span,
        }
    }
}

/// Named imports and re-exports: (file, local or exported name) -> (file, name there).
#[derive(Default)]
pub struct ImportMap(HashMap<(FileId, String), (FileId, String)>);

impl ImportMap {
    pub fn build(modules: &[ParsedModule<'_>], config: &TsConfig) -> ImportMap {
        let by_path: HashMap<PathBuf, FileId> = modules
            .iter()
            .enumerate()
            .map(|(i, m)| (m.abs.clone(), i))
            .collect();
        let mut map = HashMap::new();
        for (fid, m) in modules.iter().enumerate() {
            let resolve = |spec: &str| resolve_specifier(&m.abs, spec, config, &by_path);
            for stmt in &m.program.body {
                match stmt {
                    Statement::ImportDeclaration(imp) => {
                        let Some(target) = resolve(imp.source.value.as_str()) else {
                            continue;
                        };
                        for s in imp.specifiers.iter().flatten() {
                            let (local, imported) = match s {
                                ImportDeclarationSpecifier::ImportSpecifier(s) => {
                                    (s.local.name.as_str(), s.imported.name().as_str())
                                }
                                ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                                    (s.local.name.as_str(), "default")
                                }
                                ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => continue,
                            };
                            map.insert((fid, local.to_string()), (target, imported.to_string()));
                        }
                    }
                    Statement::ExportFromDeclaration(e) => {
                        let Some(target) = resolve(e.source.value.as_str()) else {
                            continue;
                        };
                        for s in &e.specifiers {
                            map.insert(
                                (fid, s.exported.name().as_str().to_string()),
                                (target, s.local.name().as_str().to_string()),
                            );
                        }
                    }
                    Statement::ExportNamedDeclaration(e) => {
                        for s in &e.specifiers {
                            let (local, exported) = (s.local.name(), s.exported.name());
                            if local != exported {
                                map.insert(
                                    (fid, exported.as_str().to_string()),
                                    (fid, local.as_str().to_string()),
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        ImportMap(map)
    }
}

const TS_EXTENSIONS: [&str; 5] = ["ts", "tsx", "d.ts", "mts", "cts"];

/// The module a specifier names, trying `.ts .tsx .d.ts .mts .cts` and `/index.*`.
fn resolve_specifier(
    from: &Path,
    spec: &str,
    config: &TsConfig,
    by_path: &HashMap<PathBuf, FileId>,
) -> Option<FileId> {
    let bases = if spec.starts_with('.') {
        vec![normalize(&from.parent()?.join(spec))]
    } else {
        config.candidates(spec)
    };
    for base in bases {
        // ESM-style `./x.js` names the `.ts` source.
        let stem = base
            .extension()
            .and_then(|e| e.to_str())
            .filter(|e| ["js", "jsx", "mjs", "cjs"].contains(e))
            .map(|_| base.with_extension(""))
            .unwrap_or_else(|| base.clone());
        let mut tries = vec![base.clone()];
        let stem_str = stem.to_string_lossy().into_owned();
        for ext in TS_EXTENSIONS {
            tries.push(PathBuf::from(format!("{stem_str}.{ext}")));
        }
        for ext in TS_EXTENSIONS {
            tries.push(base.join(format!("index.{ext}")));
        }
        if let Some(&id) = tries.iter().find_map(|t| by_path.get(t)) {
            return Some(id);
        }
    }
    None
}

/// Every `type`, `interface` and `enum` declared at the top level of any
/// module, keyed by (file, name), with `ImportMap` for cross-file references.
pub struct TypeIndex<'a> {
    modules: &'a [ParsedModule<'a>],
    decls: HashMap<(FileId, String), TypeDecl<'a>>,
    imports: ImportMap,
    /// Names declared at the top level of a script (a file with no import or
    /// export, such as an ambient `.d.ts`), which are global. `None`: declared
    /// in more than one script.
    globals: HashMap<String, Option<FileId>>,
}

impl<'a> TypeIndex<'a> {
    pub fn build(modules: &'a [ParsedModule<'a>], config: &TsConfig) -> TypeIndex<'a> {
        let mut decls = HashMap::new();
        let mut globals: HashMap<String, Option<FileId>> = HashMap::new();
        for (fid, m) in modules.iter().enumerate() {
            let is_script = !m.program.body.iter().any(|s| {
                s.is_module_declaration() || matches!(s, Statement::TSImportEqualsDeclaration(_))
            });
            let mut add = |name: &str, decl: TypeDecl<'a>| {
                if is_script {
                    globals
                        .entry(name.to_string())
                        .and_modify(|g| {
                            if *g != Some(fid) {
                                *g = None;
                            }
                        })
                        .or_insert(Some(fid));
                }
                decls.insert((fid, name.to_string()), decl);
            };
            for stmt in &m.program.body {
                let declaration = match stmt {
                    Statement::ExportDeclaration(e) => Some(&e.declaration),
                    Statement::ExportDefaultDeclaration(e) => {
                        if let ExportDefaultDeclarationKind::TSInterfaceDeclaration(i) = &e.declaration {
                            add("default", TypeDecl::Interface(i));
                            add(i.id.name.as_str(), TypeDecl::Interface(i));
                        }
                        None
                    }
                    stmt => stmt.as_declaration(),
                };
                match declaration {
                    Some(Declaration::TSTypeAliasDeclaration(a)) if a.type_parameters.is_none() => {
                        add(a.id.name.as_str(), TypeDecl::Alias(&a.type_annotation));
                    }
                    Some(Declaration::TSInterfaceDeclaration(i)) if i.type_parameters.is_none() => {
                        add(i.id.name.as_str(), TypeDecl::Interface(i));
                    }
                    Some(Declaration::TSEnumDeclaration(e)) => add(e.id.name.as_str(), TypeDecl::Enum(e)),
                    _ => {}
                }
            }
        }
        TypeIndex {
            modules,
            decls,
            imports: ImportMap::build(modules, config),
            globals,
        }
    }

    /// Where `name` is declared when `file` neither declares nor imports it.
    fn fallback(&self, file: FileId, name: &str) -> Option<(FileId, String)> {
        let key = (file, name.to_string());
        self.imports.0.get(&key).cloned().or_else(|| {
            let global = (*self.globals.get(name)?)?;
            (global != file).then(|| (global, key.1))
        })
    }

    /// The declaration `name` refers to in `file`, through imports:
    /// (declaring file, declared name, line).
    pub fn declaration(&self, file: FileId, name: &str) -> Option<(FileId, String, u32)> {
        let mut key = (file, name.to_string());
        let mut visited = Vec::new();
        loop {
            if visited.contains(&key) {
                return None;
            }
            if let Some(d) = self.decls.get(&key) {
                let line = self.modules[key.0].lines.line(d.span().start);
                return Some((key.0, key.1, line));
            }
            visited.push(key.clone());
            key = self.fallback(key.0, &key.1)?;
        }
    }

    /// The shape of a type expression written in `file`.
    pub fn resolve(&self, file: FileId, ty: &TSType<'a>) -> Shape {
        self.resolve_type(file, ty, &mut Vec::new())
    }

    fn resolve_type(&self, file: FileId, ty: &TSType<'a>, visiting: &mut Vec<(FileId, String)>) -> Shape {
        match ty {
            TSType::TSStringKeyword(_)
            | TSType::TSNumberKeyword(_)
            | TSType::TSBooleanKeyword(_)
            | TSType::TSBigIntKeyword(_) => Shape::Primitive,
            TSType::TSLiteralType(l) => literal_shape(&l.literal),
            TSType::TSParenthesizedType(p) => self.resolve_type(file, &p.type_annotation, visiting),
            TSType::TSArrayType(a) => self.resolve_type(file, &a.element_type, visiting),
            TSType::TSUnionType(u) => {
                let mut nullable = false;
                let mut members = Vec::new();
                for t in &u.types {
                    if is_null_type(t) {
                        nullable = true;
                    } else {
                        members.push(self.resolve_type(file, t, visiting));
                    }
                }
                Shape::union(members, nullable)
            }
            TSType::TSTypeLiteral(l) => Shape::Object(self.fields(file, &l.members, visiting)),
            TSType::TSTypeReference(r) if r.type_arguments.is_none() => match &r.type_name {
                TSTypeName::IdentifierReference(id) => self.resolve_name(file, id.name.as_str(), visiting),
                _ => Shape::Unknown,
            },
            _ => Shape::Unknown,
        }
    }

    fn resolve_name(&self, file: FileId, name: &str, visiting: &mut Vec<(FileId, String)>) -> Shape {
        let key = (file, name.to_string());
        if visiting.contains(&key) {
            return Shape::Unknown;
        }
        visiting.push(key.clone());
        let shape = if let Some(d) = self.decls.get(&key) {
            match d {
                TypeDecl::Alias(t) => self.resolve_type(file, t, visiting),
                TypeDecl::Interface(i) => self.interface_shape(file, i, visiting),
                TypeDecl::Enum(e) => enum_shape(e),
            }
        } else if let Some((f, n)) = self.fallback(file, name) {
            self.resolve_name(f, &n, visiting)
        } else {
            Shape::Unknown
        };
        visiting.pop();
        shape
    }

    fn interface_shape(
        &self,
        file: FileId,
        decl: &TSInterfaceDeclaration<'a>,
        visiting: &mut Vec<(FileId, String)>,
    ) -> Shape {
        let mut fields: Vec<Field> = Vec::new();
        for parent in &decl.extends {
            let TSTypeName::IdentifierReference(id) = &parent.type_name else {
                return Shape::Unknown;
            };
            if parent.type_arguments.is_some() {
                return Shape::Unknown;
            }
            match self.resolve_name(file, id.name.as_str(), visiting) {
                Shape::Object(parent_fields) => {
                    for f in parent_fields {
                        replace_field(&mut fields, f);
                    }
                }
                _ => return Shape::Unknown,
            }
        }
        for f in self.fields(file, &decl.body.body, visiting) {
            replace_field(&mut fields, f);
        }
        Shape::Object(fields)
    }

    fn fields(
        &self,
        file: FileId,
        members: &[TSSignature<'a>],
        visiting: &mut Vec<(FileId, String)>,
    ) -> Vec<Field> {
        let mut fields = Vec::new();
        for m in members {
            let TSSignature::TSPropertySignature(p) = m else {
                continue;
            };
            let Some(name) = p.key.static_name() else {
                continue;
            };
            let shape = match &p.type_annotation {
                Some(a) => self.resolve_type(file, &a.type_annotation, visiting),
                None => Shape::Unknown,
            };
            replace_field(
                &mut fields,
                Field {
                    name: name.into_owned(),
                    optional: p.optional,
                    shape,
                    file,
                    line: self.modules[file].lines.line(p.span.start),
                },
            );
        }
        fields
    }
}

/// A later declaration of a property replaces an earlier one (child over parent).
fn replace_field(fields: &mut Vec<Field>, f: Field) {
    match fields.iter_mut().find(|x| x.name == f.name) {
        Some(existing) => *existing = f,
        None => fields.push(f),
    }
}

pub fn is_null_type(ty: &TSType<'_>) -> bool {
    matches!(
        ty,
        TSType::TSNullKeyword(_) | TSType::TSUndefinedKeyword(_) | TSType::TSVoidKeyword(_)
    )
}

fn literal_shape(lit: &TSLiteral<'_>) -> Shape {
    match lit {
        TSLiteral::StringLiteral(s) => Shape::Literals(vec![s.value.as_str().to_string()]),
        TSLiteral::NumericLiteral(n) => Shape::Literals(vec![numeric_text(n)]),
        TSLiteral::UnaryExpression(u) => match (&u.operator, &u.argument) {
            (UnaryOperator::UnaryNegation, Expression::NumericLiteral(n)) => {
                Shape::Literals(vec![format!("-{}", numeric_text(n))])
            }
            _ => Shape::Unknown,
        },
        TSLiteral::BooleanLiteral(_) | TSLiteral::BigIntLiteral(_) => Shape::Primitive,
        TSLiteral::TemplateLiteral(_) => Shape::Unknown,
    }
}

pub fn numeric_text(n: &NumericLiteral<'_>) -> String {
    n.raw.map(|r| r.as_str().to_string()).unwrap_or_else(|| n.value.to_string())
}

/// Enum members as choices: string members by value, numeric members by
/// value with TypeScript's auto-increment; a computed member makes the
/// enum `Unknown`.
fn enum_shape(e: &TSEnumDeclaration<'_>) -> Shape {
    let mut values = Vec::new();
    let mut next: Option<f64> = Some(0.0);
    for m in &e.body.members {
        let value = match m.initializer.as_ref().map(Expression::without_parentheses) {
            None => match next {
                Some(n) => {
                    next = Some(n + 1.0);
                    n.to_string()
                }
                None => return Shape::Unknown,
            },
            Some(Expression::StringLiteral(s)) => {
                next = None;
                s.value.as_str().to_string()
            }
            Some(Expression::NumericLiteral(n)) => {
                next = Some(n.value + 1.0);
                numeric_text(n)
            }
            Some(Expression::UnaryExpression(u)) => match (&u.operator, &u.argument) {
                (UnaryOperator::UnaryNegation, Expression::NumericLiteral(n)) => {
                    next = Some(-n.value + 1.0);
                    format!("-{}", numeric_text(n))
                }
                _ => return Shape::Unknown,
            },
            Some(Expression::TemplateLiteral(t)) if t.expressions.is_empty() => {
                next = None;
                t.quasis
                    .iter()
                    .map(|q| q.value.cooked.map(|c| c.as_str().to_string()).unwrap_or_default())
                    .collect()
            }
            Some(_) => return Shape::Unknown,
        };
        values.push(value);
    }
    Shape::Literals(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tsconfig_with_comments_and_trailing_commas() {
        let text = r#"{
  // the app
  "compilerOptions": {
    "baseUrl": "./src", /* relative to this file */
    "paths": {
      "@/*": ["*"],
      "@root/*": ["../*"],
    },
  },
}"#;
        let c = TsConfig::parse(text, Path::new("/proj")).unwrap();
        assert_eq!(
            c.candidates("@/app/utils"),
            vec![PathBuf::from("/proj/src/app/utils"), PathBuf::from("/proj/src/@/app/utils")]
        );
        assert_eq!(c.candidates("@root/json/x")[0], PathBuf::from("/proj/json/x"));
        assert_eq!(c.candidates("react"), vec![PathBuf::from("/proj/src/react")]);
    }

    #[test]
    fn tsconfig_without_paths_resolves_nothing() {
        let c = TsConfig::parse("{}", Path::new("/proj")).unwrap();
        assert_eq!(c.candidates("@/x"), Vec::<PathBuf>::new());
        assert!(TsConfig::parse("{ nope", Path::new("/proj")).is_err());
    }

    #[test]
    fn strip_jsonc_keeps_string_contents() {
        assert_eq!(
            strip_jsonc(r#"{"a": "x // y", /* c */ "b": [1, 2,], }"#),
            r#"{"a": "x // y",  "b": [1, 2] }"#
        );
    }

    fn lits(v: &[&str]) -> Shape {
        Shape::Literals(v.iter().map(|s| s.to_string()).collect())
    }

    #[test]
    fn union_rules() {
        assert_eq!(Shape::union(vec![lits(&["a"]), lits(&["b", "a"])], false), lits(&["a", "b"]));
        assert_eq!(
            Shape::union(vec![lits(&["a"])], true),
            Shape::Nullable(Box::new(lits(&["a"])))
        );
        assert_eq!(Shape::union(vec![lits(&["a"]), Shape::Unknown], false), Shape::Unknown);
        assert_eq!(
            Shape::union(vec![lits(&["a"]), Shape::Unknown], true),
            Shape::Nullable(Box::new(Shape::Unknown))
        );
        assert_eq!(Shape::union(vec![lits(&["a"]), Shape::Primitive], false), Shape::Primitive);
        assert_eq!(
            Shape::union(vec![Shape::Nullable(Box::new(Shape::Primitive)), Shape::Primitive], false),
            Shape::Nullable(Box::new(Shape::Primitive))
        );
        assert_eq!(Shape::union(vec![Shape::Object(vec![]), Shape::Primitive], false), Shape::Unknown);
        assert_eq!(Shape::union(vec![Shape::Object(vec![])], false), Shape::Object(vec![]));
    }

    #[test]
    fn requirement_projection() {
        assert_eq!(
            lits(&["a", "b"]).requirement(false),
            Requirement { nullable: Some(false), choices: Some(vec!["a".into(), "b".into()]) }
        );
        assert_eq!(
            lits(&["a"]).requirement(true),
            Requirement { nullable: Some(true), choices: Some(vec!["a".into()]) }
        );
        assert_eq!(
            Shape::Nullable(Box::new(Shape::Unknown)).requirement(false),
            Requirement { nullable: Some(true), choices: None }
        );
        assert_eq!(Shape::Unknown.requirement(false), Requirement::default());
        assert!(!Shape::Primitive.requirement(true).can_reject());
        assert!(Shape::Primitive.requirement(false).can_reject());
    }
}
