//! Statement-order walk of a function body with None-narrowing, and the
//! flow-insensitive facts collected from it (local assignments, returns).
//!
//! Narrowing (a name is known to be non-None at a program point):
//! - after `if x is None: <body that returns / raises / continues / breaks>`
//!   (also `if not x: ...`), when the `if` has no `elif` / `else`;
//! - inside `if x is not None:` / `if x:` bodies, and in the `else` branch
//!   of `if x is None:` / `if not x:`; conjunctions narrow every conjunct;
//! - after `assert x is not None` / `assert x`.
//!
//! A narrowing is dropped when the name is rebound; entering a loop drops
//! every name the loop rebinds, and after `with` / `try` / `match` blocks
//! only narrowings from before the block that the block does not rebind
//! survive.

use std::collections::{HashMap, HashSet};

use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, BoolOp, CmpOp, Expr, ExprContext, Pattern, Stmt, UnaryOp};

/// Names known to be non-None at a program point.
pub type Narrowed = HashSet<String>;

/// Callbacks of `walk_block`.
pub trait FlowVisitor<'a> {
    /// A simple (non-compound) statement, with the narrowing that holds before it.
    fn simple(&mut self, stmt: &'a Stmt, narrowed: &Narrowed);
    /// An expression evaluated by a compound statement's header (an `if` /
    /// `elif` / `while` test, a `for` iterable, a `with` item, a `match` subject).
    fn header(&mut self, expr: &'a Expr, narrowed: &Narrowed);
}

/// Walk `stmts` in order, reporting simple statements and header expressions
/// with the narrowing in effect. Returns the narrowing after the block.
/// Nested function and class definitions are not entered.
pub fn walk_block<'a, V: FlowVisitor<'a>>(
    stmts: &'a [Stmt],
    narrowed: &Narrowed,
    v: &mut V,
) -> Narrowed {
    let mut n = narrowed.clone();
    for stmt in stmts {
        match stmt {
            Stmt::If(s) => {
                v.header(&s.test, &n);
                let mut body_n = n.clone();
                body_n.extend(positive(&s.test));
                walk_block(&s.body, &body_n, v);
                let mut negated = negative(&s.test);
                for clause in &s.elif_else_clauses {
                    let mut clause_n = n.clone();
                    clause_n.extend(negated.iter().cloned());
                    match &clause.test {
                        Some(test) => {
                            v.header(test, &clause_n);
                            let mut bn = clause_n.clone();
                            bn.extend(positive(test));
                            walk_block(&clause.body, &bn, v);
                            negated.extend(negative(test));
                        }
                        None => {
                            walk_block(&clause.body, &clause_n, v);
                        }
                    }
                }
                if s.elif_else_clauses.is_empty() && always_exits(&s.body) {
                    // Only the fall-through path reaches the next statement.
                    n.extend(negative(&s.test));
                } else {
                    for name in bound_names(std::slice::from_ref(stmt)) {
                        n.remove(&name);
                    }
                }
            }
            Stmt::Assert(a) => {
                v.simple(stmt, &n);
                n.extend(positive(&a.test));
            }
            Stmt::For(f) => {
                v.header(&f.iter, &n);
                remove_all(&mut n, bound_names(std::slice::from_ref(stmt)));
                walk_block(&f.body, &n, v);
                walk_block(&f.orelse, &n, v);
            }
            Stmt::While(w) => {
                remove_all(&mut n, bound_names(std::slice::from_ref(stmt)));
                v.header(&w.test, &n);
                let mut body_n = n.clone();
                body_n.extend(positive(&w.test));
                walk_block(&w.body, &body_n, v);
                walk_block(&w.orelse, &n, v);
            }
            Stmt::With(w) => {
                for item in &w.items {
                    v.header(&item.context_expr, &n);
                }
                remove_all(&mut n, bound_names(std::slice::from_ref(stmt)));
                walk_block(&w.body, &n, v);
            }
            Stmt::Try(t) => {
                let after = {
                    let mut a = n.clone();
                    remove_all(&mut a, bound_names(std::slice::from_ref(stmt)));
                    a
                };
                walk_block(&t.body, &n, v);
                for handler in &t.handlers {
                    let ast::ExceptHandler::ExceptHandler(h) = handler;
                    walk_block(&h.body, &after, v);
                }
                walk_block(&t.orelse, &after, v);
                walk_block(&t.finalbody, &after, v);
                n = after;
            }
            Stmt::Match(m) => {
                v.header(&m.subject, &n);
                remove_all(&mut n, bound_names(std::slice::from_ref(stmt)));
                for case in &m.cases {
                    walk_block(&case.body, &n, v);
                }
            }
            Stmt::FunctionDef(f) => {
                n.remove(f.name.as_str());
            }
            Stmt::ClassDef(c) => {
                n.remove(c.name.as_str());
            }
            _ => {
                v.simple(stmt, &n);
                remove_all(&mut n, bound_names(std::slice::from_ref(stmt)));
            }
        }
    }
    n
}

fn remove_all(n: &mut Narrowed, names: HashSet<String>) {
    for name in names {
        n.remove(&name);
    }
}

/// Names known non-None when `test` is true.
pub fn positive(test: &Expr) -> Vec<String> {
    match test {
        Expr::Name(n) => vec![n.id.to_string()],
        Expr::Compare(c) => none_comparison(c, CmpOp::IsNot).into_iter().collect(),
        Expr::BoolOp(b) if matches!(b.op, BoolOp::And) => {
            b.values.iter().flat_map(positive).collect()
        }
        Expr::UnaryOp(u) if matches!(u.op, UnaryOp::Not) => negative(&u.operand),
        _ => Vec::new(),
    }
}

/// Names known non-None when `test` is false.
pub fn negative(test: &Expr) -> Vec<String> {
    match test {
        Expr::Compare(c) => none_comparison(c, CmpOp::Is).into_iter().collect(),
        Expr::BoolOp(b) if matches!(b.op, BoolOp::Or) => {
            b.values.iter().flat_map(negative).collect()
        }
        Expr::UnaryOp(u) if matches!(u.op, UnaryOp::Not) => positive(&u.operand),
        _ => Vec::new(),
    }
}

/// `x <op> None` or `None <op> x` with a single comparison: `x`.
fn none_comparison(c: &ast::ExprCompare, op: CmpOp) -> Option<String> {
    if c.ops.len() != 1 || c.ops[0] != op {
        return None;
    }
    match (c.left.as_ref(), &c.comparators[0]) {
        (Expr::Name(n), Expr::NoneLiteral(_)) | (Expr::NoneLiteral(_), Expr::Name(n)) => {
            Some(n.id.to_string())
        }
        _ => None,
    }
}

/// Whether control never falls off the end of `body` (it ends in
/// `return` / `raise` / `continue` / `break`, or an `if` whose every branch does).
pub fn always_exits(body: &[Stmt]) -> bool {
    match body.last() {
        Some(Stmt::Return(_) | Stmt::Raise(_) | Stmt::Continue(_) | Stmt::Break(_)) => true,
        Some(Stmt::If(s)) => {
            always_exits(&s.body)
                && s.elif_else_clauses.iter().any(|c| c.test.is_none())
                && s.elif_else_clauses.iter().all(|c| always_exits(&c.body))
        }
        _ => false,
    }
}

/// Whether execution can reach the end of a function body (an implicit `return None`).
pub fn falls_through(body: &[Stmt]) -> bool {
    !terminates(body)
}

fn terminates(body: &[Stmt]) -> bool {
    match body.last() {
        Some(Stmt::Return(_) | Stmt::Raise(_)) => true,
        Some(Stmt::If(s)) => {
            terminates(&s.body)
                && s.elif_else_clauses.iter().any(|c| c.test.is_none())
                && s.elif_else_clauses.iter().all(|c| terminates(&c.body))
        }
        Some(Stmt::While(w)) => {
            matches!(w.test.as_ref(), Expr::BooleanLiteral(b) if b.value)
                && !contains_break(&w.body)
        }
        Some(Stmt::With(w)) => terminates(&w.body),
        Some(Stmt::Try(t)) => {
            terminates(&t.finalbody)
                || ((terminates(&t.body) || terminates(&t.orelse))
                    && t.handlers.iter().all(|h| {
                        let ast::ExceptHandler::ExceptHandler(h) = h;
                        terminates(&h.body)
                    }))
        }
        _ => false,
    }
}

/// A `break` that exits the loop whose body is `body`.
fn contains_break(body: &[Stmt]) -> bool {
    body.iter().any(|s| match s {
        Stmt::Break(_) => true,
        Stmt::If(i) => {
            contains_break(&i.body) || i.elif_else_clauses.iter().any(|c| contains_break(&c.body))
        }
        Stmt::With(w) => contains_break(&w.body),
        Stmt::Try(t) => {
            contains_break(&t.body)
                || contains_break(&t.orelse)
                || contains_break(&t.finalbody)
                || t.handlers.iter().any(|h| {
                    let ast::ExceptHandler::ExceptHandler(h) = h;
                    contains_break(&h.body)
                })
        }
        Stmt::Match(m) => m.cases.iter().any(|c| contains_break(&c.body)),
        _ => false,
    })
}

/// Every name bound anywhere in `stmts` (assignment targets, loop and `with`
/// targets, `except` names, imports, `del`, `global` / `nonlocal`, walrus
/// targets, nested `def` / `class` names, `match` captures). Over-approximates
/// (comprehension variables are included).
pub fn bound_names(stmts: &[Stmt]) -> HashSet<String> {
    let mut c = Binders {
        names: HashSet::new(),
        imports: HashSet::new(),
        skip_simple: false,
    };
    for s in stmts {
        c.visit_stmt(s);
    }
    c.names.extend(c.imports);
    c.names
}

/// Names bound in `expr` (walrus targets, comprehension variables).
pub fn bound_names_in_expr(expr: &Expr) -> HashSet<String> {
    let mut c = Binders {
        names: HashSet::new(),
        imports: HashSet::new(),
        skip_simple: false,
    };
    c.visit_expr(expr);
    c.names
}

struct Binders {
    names: HashSet<String>,
    /// Names bound by `import` statements.
    imports: HashSet<String>,
    /// Skip targets of simple `name = value` assignments (tracked separately).
    skip_simple: bool,
}

impl<'a> Visitor<'a> for Binders {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(f) => {
                self.names.insert(f.name.to_string());
            }
            Stmt::ClassDef(c) => {
                self.names.insert(c.name.to_string());
            }
            Stmt::Import(i) => {
                for a in &i.names {
                    let local = a.asname.as_ref().map(|n| n.to_string()).unwrap_or_else(|| {
                        a.name.split('.').next().unwrap_or_default().to_string()
                    });
                    self.imports.insert(local);
                }
            }
            Stmt::ImportFrom(i) => {
                for a in &i.names {
                    self.imports
                        .insert(a.asname.as_ref().unwrap_or(&a.name).to_string());
                }
            }
            Stmt::Global(g) => self.names.extend(g.names.iter().map(|n| n.to_string())),
            Stmt::Nonlocal(g) => self.names.extend(g.names.iter().map(|n| n.to_string())),
            Stmt::Assign(a) if self.skip_simple && simple_assign_targets(a).is_some() => {
                self.visit_expr(&a.value);
            }
            Stmt::AnnAssign(a)
                if self.skip_simple && matches!(a.target.as_ref(), Expr::Name(_)) =>
            {
                if let Some(v) = &a.value {
                    self.visit_expr(v);
                }
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Name(n) if matches!(n.ctx, ExprContext::Store | ExprContext::Del) => {
                self.names.insert(n.id.to_string());
            }
            Expr::Lambda(_) => {}
            _ => visitor::walk_expr(self, expr),
        }
    }

    fn visit_except_handler(&mut self, handler: &'a ast::ExceptHandler) {
        let ast::ExceptHandler::ExceptHandler(h) = handler;
        if let Some(name) = &h.name {
            self.names.insert(name.to_string());
        }
        visitor::walk_except_handler(self, handler);
    }

    fn visit_pattern(&mut self, pattern: &'a Pattern) {
        match pattern {
            Pattern::MatchAs(p) => {
                if let Some(n) = &p.name {
                    self.names.insert(n.to_string());
                }
            }
            Pattern::MatchStar(p) => {
                if let Some(n) = &p.name {
                    self.names.insert(n.to_string());
                }
            }
            Pattern::MatchMapping(p) => {
                if let Some(n) = &p.rest {
                    self.names.insert(n.to_string());
                }
            }
            _ => {}
        }
        visitor::walk_pattern(self, pattern);
    }
}

/// `(name, value)` pairs of a simple assignment: `x = v`, `a = b = v`, or
/// `a, b = u, v` (same arity, no starred elements). `None` if any target is
/// something else.
pub fn simple_assign_targets(a: &ast::StmtAssign) -> Option<Vec<(String, &Expr)>> {
    let mut out = Vec::new();
    for target in &a.targets {
        match target {
            Expr::Name(n) => out.push((n.id.to_string(), a.value.as_ref())),
            Expr::Tuple(t) => {
                let Expr::Tuple(values) = a.value.as_ref() else {
                    return None;
                };
                if t.elts.len() != values.elts.len()
                    || values.elts.iter().any(|e| matches!(e, Expr::Starred(_)))
                {
                    return None;
                }
                for (target, value) in t.elts.iter().zip(values.elts.iter()) {
                    let Expr::Name(n) = target else { return None };
                    out.push((n.id.to_string(), value));
                }
            }
            _ => return None,
        }
    }
    Some(out)
}

/// A local assignment with the narrowing in effect where it is evaluated.
#[derive(Debug, Clone)]
pub struct Assignment<'a> {
    pub value: &'a Expr,
    pub narrowed: Narrowed,
}

/// Flow-insensitive facts about a function body.
#[derive(Debug, Default)]
pub struct FunctionFlow<'a> {
    /// `name = value` assignments per local name.
    pub assignments: HashMap<String, Vec<Assignment<'a>>>,
    /// Names bound in any other way; their values are unknown.
    pub opaque: HashSet<String>,
    /// Names bound only by function-local `import` statements (the module's
    /// import map resolves them).
    pub imported: HashSet<String>,
    /// Names other code may rebind at any call: `global` names, and names a
    /// nested function declares `nonlocal`. Never narrowed; their values are unknown.
    pub unstable: HashSet<String>,
    /// `return` statements (`None` for a bare `return`), with narrowing.
    pub returns: Vec<(Option<&'a Expr>, Narrowed)>,
    /// Whether the end of the body is reachable (implicit `return None`).
    pub falls_through: bool,
    /// Locals used other than as `**name`, `name[...]` reads or read-only
    /// dict methods; a dict bound to such a name may be mutated.
    pub escaping: HashSet<String>,
}

impl<'a> FunctionFlow<'a> {
    pub fn of(body: &'a [Stmt]) -> Self {
        let mut flow = FunctionFlow {
            falls_through: falls_through(body),
            ..Default::default()
        };
        let mut binders = Binders {
            names: HashSet::new(),
            imports: HashSet::new(),
            skip_simple: true,
        };
        for s in body {
            binders.visit_stmt(s);
        }
        flow.imported = binders
            .imports
            .difference(&binders.names)
            .cloned()
            .collect();
        flow.opaque = binders.names;
        flow.opaque.extend(binders.imports);
        walk_block(body, &Narrowed::new(), &mut flow);
        let mut uses = Uses {
            escaping: HashSet::new(),
        };
        for s in body {
            uses.visit_stmt(s);
        }
        flow.escaping = uses.escaping;
        let mut shared = SharedNames::default();
        for s in body {
            shared.visit_stmt(s);
        }
        flow.opaque.extend(shared.names.iter().cloned());
        flow.unstable = shared.names;
        flow
    }

    /// Whether `name` is a local variable of the function body (bound other
    /// than by an `import`).
    pub fn binds(&self, name: &str) -> bool {
        self.assignments.contains_key(name)
            || (self.opaque.contains(name) && !self.imported.contains(name))
    }
}

impl<'a> FlowVisitor<'a> for FunctionFlow<'a> {
    fn simple(&mut self, stmt: &'a Stmt, narrowed: &Narrowed) {
        match stmt {
            Stmt::Assign(a) => {
                if let Some(pairs) = simple_assign_targets(a) {
                    for (name, value) in pairs {
                        self.assignments.entry(name).or_default().push(Assignment {
                            value,
                            narrowed: narrowed.clone(),
                        });
                    }
                }
            }
            Stmt::AnnAssign(a) => {
                if let (Expr::Name(n), Some(value)) = (a.target.as_ref(), &a.value) {
                    self.assignments
                        .entry(n.id.to_string())
                        .or_default()
                        .push(Assignment {
                            value,
                            narrowed: narrowed.clone(),
                        });
                }
            }
            Stmt::Return(r) => self.returns.push((r.value.as_deref(), narrowed.clone())),
            _ => {}
        }
    }

    fn header(&mut self, _expr: &'a Expr, _narrowed: &Narrowed) {}
}

/// `global` names of the function, and `nonlocal` names of nested functions.
#[derive(Default)]
struct SharedNames {
    names: HashSet<String>,
    nested: usize,
}

impl<'a> Visitor<'a> for SharedNames {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::Global(g) => self.names.extend(g.names.iter().map(|n| n.to_string())),
            Stmt::Nonlocal(n) if self.nested > 0 => {
                self.names.extend(n.names.iter().map(|n| n.to_string()))
            }
            Stmt::FunctionDef(f) => {
                self.nested += 1;
                for s in &f.body {
                    self.visit_stmt(s);
                }
                self.nested -= 1;
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }
}

/// Collects names used in ways that may mutate or alias a dict.
struct Uses {
    escaping: HashSet<String>,
}

const READ_ONLY_DICT_METHODS: [&str; 5] = ["get", "keys", "values", "items", "copy"];

impl<'a> Visitor<'a> for Uses {
    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Name(n) if matches!(n.ctx, ExprContext::Load) => {
                self.escaping.insert(n.id.to_string());
            }
            Expr::Call(call) => {
                if let Expr::Attribute(attr) = call.func.as_ref() {
                    if matches!(attr.value.as_ref(), Expr::Name(_))
                        && READ_ONLY_DICT_METHODS.contains(&attr.attr.as_str())
                    {
                        // `name.get(...)` etc.: a read, not an escape.
                        for arg in call.arguments.args.iter() {
                            self.visit_expr(arg);
                        }
                        for kw in call.arguments.keywords.iter() {
                            self.visit_expr(&kw.value);
                        }
                        return;
                    }
                }
                self.visit_expr(&call.func);
                for arg in call.arguments.args.iter() {
                    self.visit_expr(arg);
                }
                for kw in call.arguments.keywords.iter() {
                    // `f(**name)` reads the dict.
                    if kw.arg.is_none() && matches!(kw.value, Expr::Name(_)) {
                        continue;
                    }
                    self.visit_expr(&kw.value);
                }
            }
            Expr::Subscript(s)
                if matches!(s.ctx, ExprContext::Load)
                    && matches!(s.value.as_ref(), Expr::Name(_)) =>
            {
                self.visit_expr(&s.slice);
            }
            _ => visitor::walk_expr(self, expr),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(src: &str) -> Vec<Stmt> {
        let stmts =
            match ruff_python_parser::parse_unchecked(src, ruff_python_parser::Mode::Module.into())
                .into_syntax()
            {
                ruff_python_ast::Mod::Module(m) => m.body,
                _ => unreachable!(),
            };
        match stmts.into_iter().next() {
            Some(Stmt::FunctionDef(f)) => f.body,
            _ => panic!("expected a function"),
        }
    }

    /// Narrowing in effect at each `use(...)` expression statement, in order.
    fn narrowing_at_uses(src: &str) -> Vec<Vec<String>> {
        struct V(Vec<Vec<String>>);
        impl<'a> FlowVisitor<'a> for V {
            fn simple(&mut self, stmt: &'a Stmt, n: &Narrowed) {
                if let Stmt::Expr(e) = stmt {
                    if let Expr::Call(c) = e.value.as_ref() {
                        if matches!(c.func.as_ref(), Expr::Name(f) if f.id.as_str() == "use") {
                            let mut names: Vec<String> = n.iter().cloned().collect();
                            names.sort();
                            self.0.push(names);
                        }
                    }
                }
            }
            fn header(&mut self, _: &'a Expr, _: &Narrowed) {}
        }
        let b = body(src);
        let mut v = V(Vec::new());
        walk_block(&b, &Narrowed::new(), &mut v);
        v.0
    }

    #[test]
    fn test_narrowing_forms() {
        let uses = narrowing_at_uses(
            "def f(x, y):\n    use()\n    if x is None:\n        raise ValueError()\n    use()\n    if y is not None and z:\n        use()\n    else:\n        use()\n    if not y:\n        return\n    use()\n",
        );
        assert_eq!(
            uses,
            vec![
                vec![],
                vec!["x".to_string()],
                vec!["x".into(), "y".into(), "z".into()],
                vec!["x".into()],
                vec!["x".into(), "y".into()],
            ]
        );
    }

    #[test]
    fn test_narrowing_else_assert_and_rebinding() {
        let uses = narrowing_at_uses(
            "def f(x):\n    if x is None:\n        use()\n    else:\n        use()\n    assert x is not None\n    use()\n    x = g()\n    use()\n",
        );
        assert_eq!(
            uses,
            vec![vec![], vec!["x".to_string()], vec!["x".into()], vec![]]
        );
    }

    #[test]
    fn test_no_narrowing_without_exit_or_across_loops() {
        let uses = narrowing_at_uses(
            "def f(x, xs):\n    if x is None:\n        log()\n    use()\n    assert x\n    for i in xs:\n        use()\n        x = h(i)\n",
        );
        assert_eq!(uses, vec![Vec::<String>::new(), vec![]]);
    }

    #[test]
    fn test_falls_through() {
        assert!(!falls_through(&body("def f(x):\n    return x\n")));
        assert!(falls_through(&body(
            "def f(x):\n    if x:\n        return 1\n"
        )));
        assert!(!falls_through(&body(
            "def f(x):\n    if x:\n        return 1\n    else:\n        raise E()\n"
        )));
        assert!(!falls_through(&body(
            "def f(x):\n    while True:\n        return 1\n"
        )));
        assert!(falls_through(&body(
            "def f(x):\n    while True:\n        break\n"
        )));
        assert!(!falls_through(&body(
            "def f(x):\n    try:\n        return 1\n    except E:\n        return 2\n"
        )));
    }

    #[test]
    fn test_flow_collects_assignments_and_opaque() {
        let b = body(
            "def f(a):\n    x = 1\n    y, z = 2, 3\n    for i in a:\n        pass\n    w = v = 4\n    (q := 5)\n    x += 1\n    d = {}\n    d['k'] = 1\n    e = {}\n    g(**e)\n    return x\n",
        );
        let flow = FunctionFlow::of(&b);
        let mut assigned: Vec<&str> = flow.assignments.keys().map(|k| k.as_str()).collect();
        assigned.sort();
        assert_eq!(assigned, ["d", "e", "v", "w", "x", "y", "z"]);
        for name in ["i", "q", "x"] {
            assert!(flow.opaque.contains(name), "{name} should be opaque");
        }
        assert!(!flow.opaque.contains("y"));
        assert!(flow.escaping.contains("d"), "subscript store may mutate d");
        assert!(!flow.escaping.contains("e"), "**e only reads e");
        assert_eq!(flow.returns.len(), 1);
    }

    #[test]
    fn test_nonlocal_and_global_names_are_unstable() {
        let b = body(
            "def f(x):\n    global G\n    y = 1\n    def g():\n        nonlocal y\n        y = None\n    g()\n    return y\n",
        );
        let flow = FunctionFlow::of(&b);
        assert!(flow.unstable.contains("y") && flow.unstable.contains("G"));
        assert!(
            flow.opaque.contains("y"),
            "a nonlocal rebinding makes y's value unknown"
        );
    }
}
