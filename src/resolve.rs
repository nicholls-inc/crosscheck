//! Module paths, imports and name resolution across the project.
//!
//! Every Python file is a module named by its dotted path relative to the
//! application root (`billing/records.py` → `billing.records`,
//! `pkg/__init__.py` → `pkg`). Imports map local names to project symbols;
//! an import path matches a project module when one is a dotted suffix of the
//! other, so `from plain_python.records import X` finds `records.py` when the
//! fixture directory itself is the application root.
//!
//! Only names that resolve to a function, class or module defined in the
//! project resolve at all; builtins and third-party names resolve to nothing,
//! which the edge discovery reads as "no edge".

use std::collections::{BTreeMap, HashMap, HashSet};

use ruff_python_ast::{Expr, Stmt};

use crate::dataclass_extractor::DataClassKind;
use crate::function_extractor::FunctionInfo;

/// Dotted module name for a path relative to the application root.
pub fn module_name(relative_path: &str) -> String {
    let path = relative_path.replace('\\', "/");
    let path = path.strip_suffix(".py").unwrap_or(&path);
    let mut parts: Vec<&str> = path
        .split('/')
        .filter(|p| !p.is_empty() && *p != ".")
        .collect();
    if parts.last() == Some(&"__init__") {
        parts.pop();
    }
    parts.join(".")
}

/// `module.name`, or just `name` for the unnamed module.
pub fn qualify(module: &str, name: &str) -> String {
    if module.is_empty() {
        name.to_string()
    } else {
        format!("{module}.{name}")
    }
}

/// What an imported local name refers to (absolute import paths).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Import {
    /// `import a.b as x` binds `x` to module `a.b`; `import a.b` binds `a` to `a`.
    Module(String),
    /// `from m import n [as x]` binds `x` to `n` inside `m`.
    Symbol { module: String, name: String },
}

/// A top-level definition in a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefKind {
    Function,
    Class,
}

/// Names a module defines and imports.
#[derive(Debug, Default, Clone)]
pub struct ModuleInfo {
    pub name: String,
    pub is_package: bool,
    pub imports: HashMap<String, Import>,
    pub star_imports: Vec<String>,
    /// Top-level `def` and `class` names; a later definition replaces an earlier one.
    pub defs: HashMap<String, DefKind>,
}

impl ModuleInfo {
    pub fn from_stmts(name: &str, is_package: bool, stmts: &[Stmt]) -> Self {
        let mut info = ModuleInfo {
            name: name.to_string(),
            is_package,
            ..Default::default()
        };
        for stmt in stmts {
            match stmt {
                Stmt::FunctionDef(f) => {
                    info.imports.remove(f.name.as_str());
                    info.defs.insert(f.name.to_string(), DefKind::Function);
                }
                Stmt::ClassDef(c) => {
                    info.imports.remove(c.name.as_str());
                    info.defs.insert(c.name.to_string(), DefKind::Class);
                }
                _ => {}
            }
        }
        collect_imports(stmts, &mut info);
        info
    }
}

/// Collect imports anywhere in the module (top level, `if`/`try` blocks,
/// function and class bodies). A name that is also defined at top level
/// keeps the definition.
fn collect_imports(stmts: &[Stmt], info: &mut ModuleInfo) {
    for stmt in stmts {
        match stmt {
            Stmt::Import(imp) => {
                for alias in &imp.names {
                    let path = alias.name.to_string();
                    let (local, target) = match &alias.asname {
                        Some(asname) => (asname.to_string(), path),
                        None => {
                            let head = path.split('.').next().unwrap_or(&path).to_string();
                            (head.clone(), head)
                        }
                    };
                    if !info.defs.contains_key(&local) {
                        info.imports.insert(local, Import::Module(target));
                    }
                }
            }
            Stmt::ImportFrom(imp) => {
                let module = absolute_import(
                    &info.name,
                    info.is_package,
                    imp.level,
                    imp.module.as_ref().map(|m| m.as_str()),
                );
                for alias in &imp.names {
                    if alias.name.as_str() == "*" {
                        info.star_imports.push(module.clone());
                        continue;
                    }
                    let local = alias.asname.as_ref().unwrap_or(&alias.name).to_string();
                    if !info.defs.contains_key(&local) {
                        info.imports.insert(
                            local,
                            Import::Symbol {
                                module: module.clone(),
                                name: alias.name.to_string(),
                            },
                        );
                    }
                }
            }
            Stmt::FunctionDef(f) => collect_imports(&f.body, info),
            Stmt::ClassDef(c) => collect_imports(&c.body, info),
            Stmt::If(s) => {
                collect_imports(&s.body, info);
                for clause in &s.elif_else_clauses {
                    collect_imports(&clause.body, info);
                }
            }
            Stmt::Try(t) => {
                collect_imports(&t.body, info);
                for handler in &t.handlers {
                    let ruff_python_ast::ExceptHandler::ExceptHandler(h) = handler;
                    collect_imports(&h.body, info);
                }
                collect_imports(&t.orelse, info);
                collect_imports(&t.finalbody, info);
            }
            Stmt::With(w) => collect_imports(&w.body, info),
            _ => {}
        }
    }
}

/// Absolute module path of `from <level dots><module> import ...` in `importer`.
pub fn absolute_import(
    importer: &str,
    is_package: bool,
    level: u32,
    module: Option<&str>,
) -> String {
    if level == 0 {
        return module.unwrap_or("").to_string();
    }
    let mut parts: Vec<&str> = importer.split('.').filter(|p| !p.is_empty()).collect();
    if !is_package {
        parts.pop();
    }
    for _ in 1..level {
        parts.pop();
    }
    let base = parts.join(".");
    match module {
        Some(m) if !m.is_empty() => qualify(&base, m),
        _ => base,
    }
}

/// How a class is recognised.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassKind {
    Django,
    Data(DataClassKind),
    Plain,
}

impl ClassKind {
    /// Whether instances have extracted fields (write targets).
    pub fn has_fields(self) -> bool {
        !matches!(self, ClassKind::Plain)
    }
}

/// A class defined in the project.
#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub qualified: String,
    pub module: String,
    pub name: String,
    pub kind: ClassKind,
    /// Extracted field names in declaration order (inherited first).
    pub fields: Vec<String>,
    /// Parameter names bound by positional constructor arguments, when known.
    pub positional: Option<Vec<String>>,
    /// Base class expressions as written.
    pub bases: Vec<Expr>,
    /// Method name → qualified function name, for methods defined in this class.
    pub methods: HashMap<String, String>,
}

/// What a name or dotted expression refers to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Symbol {
    Module(String),
    Class(String),
    Function(String),
}

/// Every module, class and function in the project.
#[derive(Debug, Default)]
pub struct ProjectIndex {
    pub modules: BTreeMap<String, ModuleInfo>,
    pub classes: HashMap<String, ClassInfo>,
    pub functions: Vec<FunctionInfo>,
    /// Qualified function name → index into `functions`.
    pub function_ids: HashMap<String, usize>,
}

const MAX_DEPTH: usize = 8;

impl ProjectIndex {
    pub fn function(&self, qualified: &str) -> Option<&FunctionInfo> {
        self.function_ids
            .get(qualified)
            .map(|&i| &self.functions[i])
    }

    pub fn class(&self, qualified: &str) -> Option<&ClassInfo> {
        self.classes.get(qualified)
    }

    /// The project module an import path refers to. Exact names win; otherwise
    /// a module whose name is a dotted suffix of the path (or vice versa). Ties
    /// go to the candidate sharing the longest package prefix with `importer`;
    /// a remaining tie resolves to nothing.
    pub fn find_module(&self, path: &str, importer: &str) -> Option<&str> {
        if path.is_empty() {
            return None;
        }
        if let Some((name, _)) = self.modules.get_key_value(path) {
            return Some(name.as_str());
        }
        let dotted_path = format!(".{path}");
        let candidates: Vec<&str> = self
            .modules
            .keys()
            .filter(|m| !m.is_empty())
            .filter(|m| m.ends_with(&dotted_path) || path.ends_with(&format!(".{m}")))
            .map(|m| m.as_str())
            .collect();
        match candidates.len() {
            0 => None,
            1 => Some(candidates[0]),
            _ => {
                let shared = |m: &str| {
                    m.split('.')
                        .zip(importer.split('.'))
                        .take_while(|(a, b)| a == b)
                        .count()
                };
                let best = candidates.iter().map(|m| shared(m)).max().unwrap_or(0);
                let top: Vec<&&str> = candidates.iter().filter(|m| shared(m) == best).collect();
                if top.len() == 1 {
                    Some(top[0])
                } else {
                    None
                }
            }
        }
    }

    /// Resolve a bare name as seen from `module`: its own definitions, then
    /// its imports, then star imports.
    pub fn lookup(&self, module: &str, name: &str) -> Option<Symbol> {
        self.lookup_in_module(module, name, 0)
    }

    fn lookup_in_module(&self, module: &str, name: &str, depth: usize) -> Option<Symbol> {
        if depth > MAX_DEPTH {
            return None;
        }
        let info = self.modules.get(module)?;
        match info.defs.get(name) {
            Some(DefKind::Function) => return Some(Symbol::Function(qualify(module, name))),
            Some(DefKind::Class) => return Some(Symbol::Class(qualify(module, name))),
            None => {}
        }
        if let Some(imp) = info.imports.get(name) {
            return self.resolve_import(module, imp, depth + 1);
        }
        for star in &info.star_imports {
            if let Some(m) = self.find_module(star, module) {
                if let Some(sym) = self.lookup_in_module(m, name, depth + 1) {
                    return Some(sym);
                }
            }
        }
        let sub = qualify(module, name);
        if self.modules.contains_key(&sub) {
            return Some(Symbol::Module(sub));
        }
        None
    }

    fn resolve_import(&self, importer: &str, imp: &Import, depth: usize) -> Option<Symbol> {
        match imp {
            Import::Module(path) => Some(Symbol::Module(path.clone())),
            Import::Symbol { module, name } => {
                if let Some(m) = self.find_module(module, importer) {
                    if let Some(sym) = self.lookup_in_module(m, name, depth) {
                        return Some(sym);
                    }
                }
                let sub = qualify(module, name);
                self.find_module(&sub, importer)
                    .map(|m| Symbol::Module(m.to_string()))
            }
        }
    }

    /// `sym.attr`.
    fn attribute(&self, importer: &str, sym: Symbol, attr: &str) -> Option<Symbol> {
        match sym {
            Symbol::Module(path) => {
                if let Some(m) = self.find_module(&path, importer) {
                    if let Some(found) = self.lookup_in_module(m, attr, 1) {
                        return Some(found);
                    }
                }
                Some(Symbol::Module(qualify(&path, attr)))
            }
            Symbol::Class(q) => self.method(&q, attr).map(Symbol::Function),
            Symbol::Function(_) => None,
        }
    }

    /// Resolve a `Name` / `Attribute` chain as seen from `module`.
    pub fn resolve_expr(&self, module: &str, expr: &Expr) -> Option<Symbol> {
        let parts = dotted_parts(expr)?;
        self.resolve_dotted(module, &parts)
    }

    pub fn resolve_dotted(&self, module: &str, parts: &[String]) -> Option<Symbol> {
        let (head, rest) = parts.split_first()?;
        let mut sym = self.lookup(module, head)?;
        for part in rest {
            sym = self.attribute(module, sym, part)?;
        }
        Some(sym)
    }

    /// A class resolved from an expression (a `Class` symbol only).
    pub fn resolve_class(&self, module: &str, expr: &Expr) -> Option<&ClassInfo> {
        match self.resolve_expr(module, expr)? {
            Symbol::Class(q) => self.classes.get(&q),
            _ => None,
        }
    }

    /// Method `name` of class `class_q`, searching project-local bases.
    pub fn method(&self, class_q: &str, name: &str) -> Option<String> {
        let mut visited = HashSet::new();
        self.method_rec(class_q, name, &mut visited)
    }

    fn method_rec(
        &self,
        class_q: &str,
        name: &str,
        visited: &mut HashSet<String>,
    ) -> Option<String> {
        if !visited.insert(class_q.to_string()) {
            return None;
        }
        let class = self.classes.get(class_q)?;
        if let Some(q) = class.methods.get(name) {
            return Some(q.clone());
        }
        for base in &class.bases {
            if let Some(Symbol::Class(bq)) = self.resolve_expr(&class.module, base) {
                if let Some(found) = self.method_rec(&bq, name, visited) {
                    return Some(found);
                }
            }
        }
        None
    }
}

/// `a.b.c` → `["a", "b", "c"]` for a Name/Attribute chain.
pub fn dotted_parts(expr: &Expr) -> Option<Vec<String>> {
    match expr {
        Expr::Name(n) => Some(vec![n.id.to_string()]),
        Expr::Attribute(attr) => {
            let mut parts = dotted_parts(&attr.value)?;
            parts.push(attr.attr.to_string());
            Some(parts)
        }
        _ => None,
    }
}

/// Display names: the short name when no other node has it, else the
/// qualified name. Input pairs are `(qualified, short)`.
pub fn display_names(nodes: &[(String, String)]) -> HashMap<String, String> {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for (_, short) in nodes {
        *counts.entry(short.as_str()).or_default() += 1;
    }
    nodes
        .iter()
        .map(|(qualified, short)| {
            let name = if counts[short.as_str()] == 1 {
                short
            } else {
                qualified
            };
            (qualified.clone(), name.clone())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> Vec<Stmt> {
        match ruff_python_parser::parse_unchecked(src, ruff_python_parser::Mode::Module.into())
            .into_syntax()
        {
            ruff_python_ast::Mod::Module(m) => m.body,
            _ => unreachable!(),
        }
    }

    fn index(files: &[(&str, &str)]) -> ProjectIndex {
        let mut idx = ProjectIndex::default();
        for (path, src) in files {
            let name = module_name(path);
            let is_package = path.ends_with("__init__.py");
            idx.modules.insert(
                name.clone(),
                ModuleInfo::from_stmts(&name, is_package, &parse(src)),
            );
        }
        idx
    }

    #[test]
    fn test_module_names() {
        assert_eq!(module_name("billing/records.py"), "billing.records");
        assert_eq!(module_name("pkg/__init__.py"), "pkg");
        assert_eq!(module_name("code.py"), "code");
        assert_eq!(module_name("a\\b.py"), "a.b");
    }

    #[test]
    fn test_relative_import_paths() {
        assert_eq!(
            absolute_import("pkg.code", false, 1, Some("helpers")),
            "pkg.helpers"
        );
        assert_eq!(
            absolute_import("pkg.sub.code", false, 2, Some("x")),
            "pkg.x"
        );
        assert_eq!(absolute_import("pkg", true, 1, Some("x")), "pkg.x");
        assert_eq!(absolute_import("pkg.code", false, 1, None), "pkg");
        assert_eq!(absolute_import("code", false, 0, Some("m.n")), "m.n");
    }

    #[test]
    fn test_import_forms() {
        let idx = index(&[
            ("helpers.py", "def label(a): pass\n"),
            ("records.py", "class Invoice: pass\n"),
            (
                "code.py",
                "import helpers\nimport helpers as h\nfrom records import Invoice as Inv\nfrom plain.records import Invoice\ndef f(): pass\n",
            ),
        ]);
        let call = |src: &str| {
            let stmts = parse(src);
            let Stmt::Expr(e) = &stmts[0] else { panic!() };
            idx.resolve_expr("code", &e.value)
        };
        assert_eq!(
            call("helpers.label"),
            Some(Symbol::Function("helpers.label".into()))
        );
        assert_eq!(
            call("h.label"),
            Some(Symbol::Function("helpers.label".into()))
        );
        assert_eq!(call("Inv"), Some(Symbol::Class("records.Invoice".into())));
        // `plain.records` matches module `records` by dotted suffix
        assert_eq!(
            call("Invoice"),
            Some(Symbol::Class("records.Invoice".into()))
        );
        assert_eq!(call("f"), Some(Symbol::Function("code.f".into())));
        assert_eq!(call("len"), None);
        assert_eq!(
            call("h.missing"),
            Some(Symbol::Module("helpers.missing".into()))
        );
    }

    #[test]
    fn test_relative_and_ambiguous_modules() {
        let idx = index(&[
            ("pkg/__init__.py", ""),
            ("pkg/helpers.py", "def boost(a): pass\n"),
            (
                "pkg/code.py",
                "from .helpers import boost\nfrom . import helpers\n",
            ),
            ("billing/records.py", "class Invoice: pass\n"),
            ("shop/records.py", "class Invoice: pass\n"),
            ("billing/code.py", "from records import Invoice\n"),
            ("other.py", "from records import Invoice\n"),
        ]);
        assert_eq!(
            idx.lookup("pkg.code", "boost"),
            Some(Symbol::Function("pkg.helpers.boost".into()))
        );
        assert_eq!(
            idx.lookup("pkg.code", "helpers"),
            Some(Symbol::Module("pkg.helpers".into()))
        );
        // Ambiguous suffix match: the importer's own package wins ...
        assert_eq!(
            idx.lookup("billing.code", "Invoice"),
            Some(Symbol::Class("billing.records.Invoice".into()))
        );
        // ... and without one it resolves to nothing.
        assert_eq!(idx.lookup("other", "Invoice"), None);
    }

    #[test]
    fn test_display_names() {
        let names = display_names(&[
            ("billing.code.make".into(), "make".into()),
            ("shop.code.make".into(), "make".into()),
            ("helpers.label".into(), "label".into()),
        ]);
        assert_eq!(names["billing.code.make"], "billing.code.make");
        assert_eq!(names["helpers.label"], "label");
    }
}
