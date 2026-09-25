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

use std::cell::OnceCell;
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

/// The `def` and `class` statements that define module-level names: those
/// at top level and those inside module-level `if` / `try` / `with` blocks
/// (`if not is_model_registered(...): class Price(...)`), in source order.
pub fn module_defs(stmts: &[Stmt]) -> Vec<&Stmt> {
    fn walk<'s>(stmts: &'s [Stmt], out: &mut Vec<&'s Stmt>) {
        for stmt in stmts {
            match stmt {
                Stmt::FunctionDef(_) | Stmt::ClassDef(_) => out.push(stmt),
                Stmt::If(s) => {
                    walk(&s.body, out);
                    for clause in &s.elif_else_clauses {
                        walk(&clause.body, out);
                    }
                }
                Stmt::Try(t) => {
                    walk(&t.body, out);
                    for handler in &t.handlers {
                        let ruff_python_ast::ExceptHandler::ExceptHandler(h) = handler;
                        walk(&h.body, out);
                    }
                    walk(&t.orelse, out);
                    walk(&t.finalbody, out);
                }
                Stmt::With(w) => walk(&w.body, out),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(stmts, &mut out);
    out
}

/// Class definitions at module level (see `module_defs`).
pub fn module_classes(stmts: &[Stmt]) -> impl Iterator<Item = &ruff_python_ast::StmtClassDef> {
    module_defs(stmts).into_iter().filter_map(|s| match s {
        Stmt::ClassDef(c) => Some(c),
        _ => None,
    })
}

/// Classes nested in a class body (at any depth), with their dotted path
/// below the outer class (`Status`, `Status.Inner`).
pub fn nested_classes(class: &ruff_python_ast::StmtClassDef) -> Vec<(String, &ruff_python_ast::StmtClassDef)> {
    let mut out = Vec::new();
    for stmt in &class.body {
        if let Stmt::ClassDef(inner) = stmt {
            out.push((inner.name.to_string(), inner));
            for (path, deeper) in nested_classes(inner) {
                out.push((format!("{}.{path}", inner.name), deeper));
            }
        }
    }
    out
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
    /// A name bound exactly once, at top level, to a literal (`Q4 = Decimal("0.0001")`).
    Constant,
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
        for stmt in module_defs(stmts) {
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
        for name in module_constants(stmts).into_keys() {
            if !info.imports.contains_key(&name) && !info.defs.contains_key(&name) {
                info.defs.insert(name, DefKind::Constant);
            }
        }
        info
    }
}

/// Whether `expr` is a literal whose facts need no context: `None`, a bool,
/// number or (non-f) string, a negated number, `Decimal("...")` /
/// `Decimal(n)`, `"x" * n`, or a tuple / list of such literals.
pub fn is_constant_literal(expr: &Expr) -> bool {
    match expr {
        Expr::NoneLiteral(_)
        | Expr::BooleanLiteral(_)
        | Expr::NumberLiteral(_)
        | Expr::StringLiteral(_) => true,
        Expr::UnaryOp(u) => {
            matches!(u.op, ruff_python_ast::UnaryOp::USub | ruff_python_ast::UnaryOp::UAdd)
                && matches!(u.operand.as_ref(), Expr::NumberLiteral(_))
        }
        // Containers of literals, and choice lists `[(value, label), ...]`
        // (the label may be any expression, e.g. `_("Active")`).
        Expr::Tuple(t) => t.elts.iter().all(|e| is_constant_literal(e) || is_choice_entry(e)),
        Expr::List(l) => l.elts.iter().all(|e| is_constant_literal(e) || is_choice_entry(e)),
        Expr::BinOp(b) => {
            matches!(b.op, ruff_python_ast::Operator::Mult)
                && matches!(
                    (b.left.as_ref(), b.right.as_ref()),
                    (Expr::StringLiteral(_), Expr::NumberLiteral(_))
                        | (Expr::NumberLiteral(_), Expr::StringLiteral(_))
                )
        }
        Expr::Call(c) => {
            let decimal = match c.func.as_ref() {
                Expr::Name(n) => n.id.as_str() == "Decimal",
                Expr::Attribute(a) => {
                    a.attr.as_str() == "Decimal"
                        && matches!(a.value.as_ref(), Expr::Name(n) if n.id.as_str() == "decimal")
                }
                _ => false,
            };
            decimal
                && c.arguments.keywords.is_empty()
                && c.arguments.args.len() == 1
                && matches!(
                    &c.arguments.args[0],
                    Expr::StringLiteral(_) | Expr::NumberLiteral(_) | Expr::UnaryOp(_)
                )
                && is_constant_literal(&c.arguments.args[0])
        }
        _ => false,
    }
}

/// `(value, label)` with a literal value, or a named group `(label, [...])`.
fn is_choice_entry(expr: &Expr) -> bool {
    let elts: &[Expr] = match expr {
        Expr::Tuple(t) => &t.elts[..],
        Expr::List(l) => &l.elts[..],
        _ => return false,
    };
    match elts {
        // The value may also name a constant (`(OPEN, "Open")`).
        [value, rest] => {
            is_constant_literal(value)
                || dotted_parts(value).is_some()
                || matches!(rest, Expr::List(_) | Expr::Tuple(_)) && is_constant_literal(rest)
        }
        _ => false,
    }
}

/// Module-level names that stand for an annotation: type aliases whose value
/// is a union or `Optional` (`T_REQUESTOR = App | User | None`,
/// `Maybe = Optional[int]`), and type variables (`N = TypeVar("N")`, which
/// stand for any type: `Any`). Name -> the annotation to read instead.
pub fn annotation_aliases(stmts: &[Stmt]) -> HashMap<String, Expr> {
    let any = || {
        ruff_python_parser::parse_expression("Any")
            .ok()
            .map(|p| p.into_expr())
    };
    let mut out = HashMap::new();
    for (name, value) in single_assignments(stmts) {
        let alias = match &value {
            Expr::BinOp(b) if matches!(b.op, ruff_python_ast::Operator::BitOr) => Some(value.clone()),
            Expr::Subscript(s)
                if dotted_parts(&s.value)
                    .is_some_and(|p| matches!(p.last().map(String::as_str), Some("Optional" | "Union"))) =>
            {
                Some(value.clone())
            }
            Expr::Call(c)
                if dotted_parts(&c.func)
                    .is_some_and(|p| p.last().map(String::as_str) == Some("TypeVar")) =>
            {
                any()
            }
            _ => None,
        };
        if let Some(a) = alias {
            out.insert(name, a);
        }
    }
    out
}

/// Whether `expr` is `re.compile(...)` (a compiled regular expression).
pub fn is_re_compile(expr: &Expr) -> bool {
    matches!(expr, Expr::Call(c)
        if dotted_parts(&c.func).is_some_and(|p| p == ["re", "compile"] || p == ["regex", "compile"]))
}

/// Names bound exactly once in `stmts` (as for `module_constants`) to a
/// compiled regular expression (`CODE = re.compile(...)`).
pub fn module_patterns(stmts: &[Stmt]) -> Vec<String> {
    single_assignments(stmts)
        .into_iter()
        .filter(|(_, v)| is_re_compile(v))
        .map(|(n, _)| n)
        .collect()
}

/// Names bound exactly once in `stmts` (not counting nested function and
/// class bodies), by a simple `name = literal` / `name: T = literal`, and
/// never declared `global` in a function: name -> the literal.
pub fn module_constants(stmts: &[Stmt]) -> HashMap<String, Expr> {
    single_assignments(stmts)
        .into_iter()
        .filter(|(_, v)| is_constant_literal(v))
        .collect()
}

/// Names bound exactly once in `stmts` by a simple assignment (and in no
/// other way, and never declared `global`), in source order, with the value.
fn single_assignments(stmts: &[Stmt]) -> Vec<(String, Expr)> {
    let flow = crate::flow::FunctionFlow::of(stmts);
    let globals = global_names(stmts);
    let mut out: Vec<(String, Expr, u32)> = flow
        .assignments
        .iter()
        .filter(|(name, assigns)| {
            assigns.len() == 1 && !flow.opaque.contains(*name) && !globals.contains(*name)
        })
        .map(|(name, assigns)| {
            use ruff_text_size::Ranged;
            (
                name.clone(),
                assigns[0].value.clone(),
                assigns[0].value.range().start().to_u32(),
            )
        })
        .collect();
    out.sort_by(|a, b| (a.2, &a.0).cmp(&(b.2, &b.0)));
    out.into_iter().map(|(n, v, _)| (n, v)).collect()
}

/// Names any function in `stmts` (at any depth) declares `global`.
fn global_names(stmts: &[Stmt]) -> HashSet<String> {
    use ruff_python_ast::visitor::{self, Visitor};
    struct G(HashSet<String>);
    impl<'a> Visitor<'a> for G {
        fn visit_stmt(&mut self, stmt: &'a Stmt) {
            if let Stmt::Global(g) = stmt {
                self.0.extend(g.names.iter().map(|n| n.to_string()));
            }
            visitor::walk_stmt(self, stmt);
        }
    }
    let mut g = G(HashSet::new());
    for s in stmts {
        g.visit_stmt(s);
    }
    g.0
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
    /// Class-body constants (`FROZEN = "frozen"`, `STATUS = [...]`, enum members).
    pub constants: HashMap<String, Expr>,
    /// Attributes the class's own methods assign through `self` (instance
    /// state that may shadow a class constant).
    pub stored_attrs: HashSet<String>,
    /// For enum-like classes (Django `TextChoices` / `IntegerChoices`, `enum.Enum`):
    /// how member values are read.
    pub enum_kind: Option<EnumKind>,
    /// Whether instances are validated strictly (pydantic strict mode):
    /// no numeric coercion.
    pub strict: bool,
    /// Names of `constants` in class-body order.
    pub constant_order: Vec<String>,
    /// Declared nullability of extracted fields (`null=True/False`,
    /// `Optional[T]` or not), when known.
    pub field_nullable: HashMap<String, bool>,
    /// Class-body names bound once to `re.compile(...)`.
    pub patterns: HashSet<String>,
}

/// An enum-like class: members are class-body constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnumKind {
    /// Django `TextChoices` / `IntegerChoices` / `Choices`: `NAME = value` or
    /// `NAME = value, label`; `.choices` lists the values.
    DjangoChoices,
    /// `enum.Enum` and friends: `NAME = value`.
    Enum,
}

impl ClassInfo {
    /// A plain class with no recognised fields.
    pub fn new(module: &str, name: &str, bases: Vec<Expr>) -> Self {
        ClassInfo {
            qualified: qualify(module, name),
            module: module.to_string(),
            name: name.to_string(),
            kind: ClassKind::Plain,
            fields: Vec::new(),
            positional: None,
            bases,
            methods: HashMap::new(),
            constants: HashMap::new(),
            stored_attrs: HashSet::new(),
            enum_kind: None,
            strict: false,
            constant_order: Vec::new(),
            field_nullable: HashMap::new(),
            patterns: HashSet::new(),
        }
    }

    /// Record the class body: constants, `self.x = ...` stores, enum kind.
    pub fn with_body(mut self, body: &[Stmt]) -> Self {
        self.stored_attrs = self_stores(body);
        self.enum_kind = self.bases.iter().find_map(|b| {
            let parts = dotted_parts(b)?;
            match parts.last()?.as_str() {
                "TextChoices" | "IntegerChoices" | "Choices" => Some(EnumKind::DjangoChoices),
                "Enum" | "StrEnum" | "IntEnum" | "IntFlag" | "Flag" => Some(EnumKind::Enum),
                _ => None,
            }
        });
        let is_enum = self.enum_kind.is_some();
        self.patterns = module_patterns(body).into_iter().collect();
        for (name, value) in single_assignments(body) {
            // Enum members `NAME = value, label` may have a non-literal label.
            let member = is_enum
                && matches!(&value, Expr::Tuple(t) if t.elts.first().is_some_and(is_constant_literal));
            if member || is_constant_literal(&value) {
                self.constant_order.push(name.clone());
                self.constants.insert(name, value);
            }
        }
        self
    }

    /// The value of enum member `name` (for Django choices, the first element
    /// of `value, label`).
    pub fn member_value(&self, name: &str) -> Option<&Expr> {
        let kind = self.enum_kind?;
        let expr = self.constants.get(name)?;
        match (kind, expr) {
            (EnumKind::DjangoChoices, Expr::Tuple(t)) if !t.elts.is_empty() => Some(&t.elts[0]),
            _ => Some(expr),
        }
    }

    /// Values of every enum member, in declaration order, as choice strings
    /// (`None` if any member value is not a string or integer literal).
    pub fn member_choices(&self) -> Option<Vec<String>> {
        let mut out = Vec::new();
        for name in &self.constant_order {
            if name.starts_with('_') || name.chars().next().is_some_and(|c| c.is_lowercase()) {
                continue;
            }
            out.push(literal_choice(self.member_value(name)?)?);
        }
        (!out.is_empty()).then_some(out)
    }
}

/// A string or integer literal as a choice value (`"a"` -> `a`, `3` -> `3`).
pub fn literal_choice(expr: &Expr) -> Option<String> {
    match expr {
        Expr::StringLiteral(s) => Some(s.value.to_string()),
        Expr::NumberLiteral(n) => match &n.value {
            ruff_python_ast::Number::Int(i) => i.as_i64().map(|v| v.to_string()),
            _ => None,
        },
        Expr::UnaryOp(u) if matches!(u.op, ruff_python_ast::UnaryOp::USub) => {
            literal_choice(&u.operand).filter(|s| s.chars().all(|c| c.is_ascii_digit())).map(|s| format!("-{s}"))
        }
        _ => None,
    }
}

/// Attributes assigned as `<first param>.attr = ...` in the methods of a class body.
fn self_stores(body: &[Stmt]) -> HashSet<String> {
    use ruff_python_ast::visitor::{self, Visitor};
    use ruff_python_ast::ExprContext;
    struct S<'n> {
        receiver: &'n str,
        out: HashSet<String>,
    }
    impl<'a> Visitor<'a> for S<'_> {
        fn visit_expr(&mut self, expr: &'a Expr) {
            if let Expr::Attribute(a) = expr {
                if matches!(a.ctx, ExprContext::Store | ExprContext::Del)
                    && matches!(a.value.as_ref(), Expr::Name(n) if n.id.as_str() == self.receiver)
                {
                    self.out.insert(a.attr.to_string());
                }
            }
            visitor::walk_expr(self, expr);
        }
    }
    let mut out = HashSet::new();
    for stmt in body {
        if let Stmt::FunctionDef(f) = stmt {
            let Some(first) = f.parameters.posonlyargs.iter().chain(f.parameters.args.iter()).next()
            else {
                continue;
            };
            let mut s = S {
                receiver: first.parameter.name.as_str(),
                out: HashSet::new(),
            };
            for st in &f.body {
                s.visit_stmt(st);
            }
            out.extend(s.out);
        }
    }
    out
}

/// What a name or dotted expression refers to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Symbol {
    Module(String),
    Class(String),
    Function(String),
    /// A module constant (`module.NAME`) or class constant (`module.Class.NAME`).
    Constant(String),
}

/// Every module, class and function in the project.
#[derive(Debug, Default)]
pub struct ProjectIndex {
    pub modules: BTreeMap<String, ModuleInfo>,
    pub classes: HashMap<String, ClassInfo>,
    pub functions: Vec<FunctionInfo>,
    /// Qualified function name → index into `functions`.
    pub function_ids: HashMap<String, usize>,
    /// Module constants by qualified name (`module.NAME`).
    pub constants: HashMap<String, Expr>,
    /// Module-level names bound once to `re.compile(...)`, qualified.
    pub patterns: HashSet<String>,
    /// Modules by last dotted segment, for suffix matching (built on first use).
    by_last_segment: OnceCell<HashMap<String, Vec<String>>>,
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
        // A dotted suffix match in either direction shares the last segment.
        let by_last = self.by_last_segment.get_or_init(|| {
            let mut map: HashMap<String, Vec<String>> = HashMap::new();
            for m in self.modules.keys().filter(|m| !m.is_empty()) {
                let last = m.rsplit('.').next().unwrap_or(m);
                map.entry(last.to_string()).or_default().push(m.clone());
            }
            map
        });
        let last = path.rsplit('.').next().unwrap_or(path);
        let candidates: Vec<&str> = by_last
            .get(last)
            .into_iter()
            .flatten()
            .filter(|m| m.ends_with(&dotted_path) || path.ends_with(&format!(".{m}")))
            .filter_map(|m| self.modules.get_key_value(m.as_str()).map(|(k, _)| k.as_str()))
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
            Some(DefKind::Constant) => return Some(Symbol::Constant(qualify(module, name))),
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
            Symbol::Class(q) => self
                .method(&q, attr)
                .map(Symbol::Function)
                .or_else(|| self.class_constant(&q, attr).map(Symbol::Constant))
                .or_else(|| self.nested_class(&q, attr).map(Symbol::Class)),
            Symbol::Function(_) | Symbol::Constant(_) => None,
        }
    }

    /// Resolve a `Name` / `Attribute` chain as seen from `module`.
    pub fn resolve_expr(&self, module: &str, expr: &Expr) -> Option<Symbol> {
        // A parameterised generic (`Base[T]`) names its class.
        if let Expr::Subscript(s) = expr {
            return match self.resolve_expr(module, &s.value)? {
                sym @ Symbol::Class(_) => Some(sym),
                _ => None,
            };
        }
        let parts = dotted_parts(expr)?;
        self.resolve_dotted(module, &parts)
    }

    pub fn resolve_dotted(&self, module: &str, parts: &[String]) -> Option<Symbol> {
        let (head, rest) = parts.split_first()?;
        let sym = self.lookup(module, head)?;
        self.resolve_attrs(module, sym, rest)
    }

    /// `sym.a.b...` for attribute names `rest`, as seen from `module`.
    pub fn resolve_attrs(&self, module: &str, mut sym: Symbol, rest: &[String]) -> Option<Symbol> {
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

    /// Whether bare `name` in `module` is a module-level compiled pattern
    /// (defined there or imported from where it is defined).
    pub fn is_pattern(&self, module: &str, name: &str) -> bool {
        if self.patterns.contains(&qualify(module, name)) {
            return true;
        }
        match self.modules.get(module).and_then(|m| m.imports.get(name)) {
            Some(Import::Symbol { module: m, name: n }) => self
                .find_module(m, module)
                .is_some_and(|found| self.patterns.contains(&qualify(found, n))),
            _ => false,
        }
    }

    /// The literal a constant symbol is bound to.
    pub fn constant(&self, qualified: &str) -> Option<&Expr> {
        if let Some(e) = self.constants.get(qualified) {
            return Some(e);
        }
        let (class_q, name) = qualified.rsplit_once('.')?;
        self.classes.get(class_q)?.constants.get(name)
    }

    /// Class constant `name` of class `class_q` (searching project-local
    /// bases): its qualified name `defining_class.name`.
    pub fn class_constant(&self, class_q: &str, name: &str) -> Option<String> {
        let mut visited = HashSet::new();
        let mut stack = vec![class_q.to_string()];
        while let Some(q) = stack.pop() {
            if visited.len() > 64 || !visited.insert(q.clone()) {
                continue;
            }
            let Some(class) = self.classes.get(&q) else { continue };
            if class.methods.contains_key(name) {
                return None;
            }
            if class.constants.contains_key(name) {
                return Some(qualify(&q, name));
            }
            for base in class.bases.iter().rev() {
                if let Some(Symbol::Class(bq)) = self.resolve_expr(&class.module, base) {
                    stack.push(bq);
                }
            }
        }
        None
    }

    /// Class `name` nested in the body of class `class_q` (or of a
    /// project-local base): its qualified name.
    pub fn nested_class(&self, class_q: &str, name: &str) -> Option<String> {
        let mut visited = HashSet::new();
        let mut stack = vec![class_q.to_string()];
        while let Some(q) = stack.pop() {
            if visited.len() > 64 || !visited.insert(q.clone()) {
                continue;
            }
            let nested = qualify(&q, name);
            if self.classes.contains_key(&nested) {
                return Some(nested);
            }
            let Some(class) = self.classes.get(&q) else { continue };
            for base in class.bases.iter().rev() {
                if let Some(Symbol::Class(bq)) = self.resolve_expr(&class.module, base) {
                    stack.push(bq);
                }
            }
        }
        None
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
            ruff_python_ast::Mod::Module(m) => m.body.to_vec(),
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
