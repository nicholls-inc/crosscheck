//! Statement-order walk of a function body with None-narrowing and
//! reaching definitions, and the facts collected from it (local
//! assignments, returns).
//!
//! Narrowing (a name is known to be non-None at a program point):
//! - after `if x is None: <body that returns / raises / continues / breaks>`
//!   (also `if not x: ...`), when the `if` has no `elif` / `else`;
//! - inside `if x is not None:` / `if x:` bodies, and in the `else` branch
//!   of `if x is None:` / `if not x:`; conjunctions narrow every conjunct;
//! - inside `if isinstance(x, str):` and after
//!   `if not isinstance(x, str): return`, for the calls in
//!   `Exits::excludes_none` only; on an attribute, until a caller's guard
//!   on it would end (`Effects::hands_off`);
//! - after `assert x is not None` / `assert x`;
//! - inside `if x in {"a", "b"}:` (a set, list or tuple display of string,
//!   bytes, number or bool literals), and after `if x not in (...): return`.
//!
//! Membership: `k in d` (a name or string literal `k`, a name or `obj.f`
//! `d`) narrows like a test and records `member_key(d, k)` among the names,
//! so `d.get(k)` there is `d[k]` when `d` is known to be a dict (see
//! `value_analysis`). The fact is dropped when `d` or `k` is rebound, and
//! after any statement, header expression or operand (see `Effects`) that
//! calls anything other than a `.get` on `d` itself, since a call may remove
//! the key.
//!
//! Parameters: a parameter is untouched until a statement may hand its
//! object to other code (passes it, calls a method on it, stores it); a
//! caller's guard on `p.f` holds only while `p` is untouched.
//!
//! Reaching definitions: after a simple assignment `x = v` the value of `x`
//! is that assignment's; after an `if`, the definitions of every branch that
//! falls through (each with whether `x` is non-None on that path). So
//! `v = d.get(k); if v is None: v = "x"` leaves `v` non-None. After a `try`,
//! the join of the body followed by `else` and of each handler that falls
//! through; a handler starts from the state before the `try` without the
//! names the `try` binds, and a `finally` starts from the join of that
//! state and the fall-through join.
//!
//! A narrowing (and the definition set) is dropped when the name is rebound
//! other than by a simple assignment; entering a loop drops every name the
//! loop rebinds, and after a `match` only narrowings from before it that it
//! does not rebind survive. A `with` body is walked like straight-line code,
//! but what it narrowed does not survive it: its context manager may suppress
//! an exception raised anywhere in it (`contextlib.suppress`, `pytest.raises`).
//! For the same reason a `with` ends the flow only when its body ends in
//! `return`, never in `raise` or a call that never returns. An exception
//! raised by a statement before that `return`, or by its value, is not yet
//! reached: the extractor does not know which managers suppress. A name whose
//! definition set is not known takes the join of every assignment.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, BoolOp, CmpOp, Expr, ExprContext, Pattern, Stmt, UnaryOp};
use ruff_text_size::Ranged;

/// What resolution decided about the nodes of one function body, by start
/// byte offset (filled by `exits::function_exits`, which can see what a name
/// resolves to and what members an enum has): the statements that never
/// complete, and the calls that rule out None. Any other statement is assumed
/// to complete, and any other call narrows nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Exits {
    /// Expression statements whose call never returns.
    pub calls: HashSet<u32>,
    /// `match` statements whose cases cover every member of the subject's enum.
    pub matches: HashSet<u32>,
    /// `isinstance(s, spec)` calls that are true only when `s` is not None:
    /// the builtin `isinstance`, with a spec of builtin classes that exclude None.
    pub excludes_none: HashSet<u32>,
}

impl Exits {
    fn is_exit_call(&self, stmt: &Stmt) -> bool {
        matches!(stmt, Stmt::Expr(_)) && self.calls.contains(&stmt.start().to_u32())
    }

    fn is_exhaustive(&self, m: &ast::StmtMatch) -> bool {
        match_is_exhaustive(m) || self.matches.contains(&m.start().to_u32())
    }
}

/// Where a local's value can come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Def {
    /// The parameter's argument (the name was not rebound on this path); for
    /// an attribute `obj.f`, a value not assigned as `obj.f = ...` here (from
    /// before the function, or from the construction of a rebound `obj`).
    Param,
    /// The `i`-th simple assignment to the name, in walk order
    /// (`FunctionFlow::assignments[name][i]`).
    Assign(usize),
    /// An augmented assignment (`x += v`): some value, never None.
    Augmented,
}

/// What is known about names at a program point.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Narrowed {
    /// Names known to be non-None.
    names: HashSet<String>,
    /// Reaching definitions of locals, each with whether the value is known
    /// non-None on its path. A name absent here may have any of its values.
    defs: HashMap<String, Vec<(Def, bool)>>,
    /// Parameters whose objects no code has been handed on any path here
    /// (see `Effects::reached`): their fields still hold the values the
    /// caller passed.
    untouched: HashSet<String>,
    /// Parameters of which no attribute (`p.a`) has been handed to other
    /// code as a value on any path here (see `Effects::handed`): code that
    /// holds `p.a` can write `p.a.f`, which `untouched` does not see.
    links_untouched: HashSet<String>,
    /// The chains `p.a.b` (two or more attributes deep) read as a value
    /// somewhere on a path here (see `Effects::handed_paths`): the object
    /// `p.a.b` may be in other code's hands, which can write its fields. A
    /// caller's guard on a longer chain `p.a.b.f` holds only while no proper
    /// prefix of it is here.
    handed_paths: HashSet<String>,
    /// The attributes in `names` known non-None only by an `isinstance` test
    /// (`Fact::held`). Each ends where a caller's guard on it would end
    /// (`Effects::hands_off`), counted from the test.
    held: HashSet<String>,
    /// The names in `names` known non-None only by an `isinstance` test
    /// (`Fact::by_isinstance`): a walrus that rebinds one ends it
    /// (`Effects::rebinds`). A superset of `held`.
    iso: HashSet<String>,
    /// The roots `x` of which `x.__dict__` (or `x.a.__dict__`) was read on
    /// some path here (see `Effects::exposes`): a table that code can keep and
    /// write to later, which `untouched` does not see.
    exposed: HashSet<String>,
    /// The function's `Exits::excludes_none`, for `positive`.
    excludes_none: Rc<HashSet<u32>>,
}

impl Narrowed {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether `name` is known non-None here.
    pub fn contains(&self, name: &str) -> bool {
        self.names.contains(name)
            || self
                .defs
                .get(name)
                .is_some_and(|d| !d.is_empty() && d.iter().all(|(_, nn)| *nn))
    }

    /// Names known non-None (for tests and diagnostics).
    pub fn names(&self) -> impl Iterator<Item = &String> {
        self.names.iter()
    }

    /// The definitions of `name` that reach this point, when known.
    pub fn defs(&self, name: &str) -> Option<&[(Def, bool)]> {
        self.defs.get(name).map(Vec::as_slice)
    }

    /// Mark names non-None.
    pub fn extend(&mut self, names: impl IntoIterator<Item = String>) {
        for name in names {
            if let Some(d) = self.defs.get_mut(&name) {
                for entry in d.iter_mut() {
                    entry.1 = true;
                }
            }
            self.held.remove(&name);
            self.iso.remove(&name);
            self.names.insert(name);
        }
    }

    /// Mark the names of `facts` non-None. An `isinstance` fact does not
    /// weaken a name already known non-None by another test.
    pub fn assume(&mut self, facts: impl IntoIterator<Item = Fact>) {
        for f in facts {
            if !f.by_isinstance {
                self.extend([f.name]);
                continue;
            }
            let known_otherwise = self.names.contains(&f.name) && !self.iso.contains(&f.name);
            if known_otherwise {
                continue;
            }
            if f.held {
                self.names.insert(f.name.clone());
                self.held.insert(f.name.clone());
            } else {
                self.extend([f.name.clone()]);
            }
            self.iso.insert(f.name);
        }
    }

    /// Forget everything about `name` (rebound in an unknown way), and
    /// about its attributes (`name.f`) when `name` is a variable.
    pub fn remove(&mut self, name: &str) {
        self.names.remove(name);
        self.held.remove(name);
        self.iso.remove(name);
        self.defs.remove(name);
        self.forget_attributes(name);
        self.forget_members_of(name);
    }

    /// `name` now holds the value of `def`.
    pub fn rebind(&mut self, name: &str, def: Def) {
        self.names.remove(name);
        self.held.remove(name);
        self.iso.remove(name);
        self.defs.insert(name.to_string(), vec![(def, false)]);
        self.forget_attributes(name);
        self.forget_members_of(name);
    }

    /// Drop every `member_key` fact whose container is `name` (or an
    /// attribute of it) or whose key is `name`.
    fn forget_members_of(&mut self, name: &str) {
        let prefix = format!("{name}.");
        self.names.retain(|n| match n.split_once('[') {
            Some((container, key)) => {
                container != name && !container.starts_with(&prefix) && key.strip_suffix(']') != Some(name)
            }
            None => true,
        });
    }

    /// Whether no code has been handed parameter `name`'s object on any
    /// path to here.
    pub fn untouched(&self, name: &str) -> bool {
        self.untouched.contains(name)
    }

    /// Whether no attribute of parameter `name` (`name.a`) has been handed
    /// to other code as a value on any path to here: a caller's guard on a
    /// chain `name.a.f` holds only then.
    pub fn links_untouched(&self, name: &str) -> bool {
        self.links_untouched.contains(name)
    }

    /// Whether an object along `chain` (`p.a.b.f`, a name and attributes) has
    /// been read as a value on some path to here: a proper prefix of `chain`
    /// of two or more attributes (`p.a.b`) was passed on, stored or returned,
    /// so other code can write the field. Reading `chain` itself does not
    /// count, nor does reading a field of it.
    pub fn link_handed(&self, chain: &str) -> bool {
        self.handed_paths.iter().any(|h| is_link_of(h, chain))
    }

    /// Whether a caller's guard on `chain` (`p.f`, `p.a.f`) would hold here,
    /// counting from the function's entry (`Scope::field_read_facts`): `p` is
    /// still the parameter's own value, no code was handed it, nor, for a
    /// chain, an attribute of it or a proper prefix of the chain, and no
    /// `p.__dict__` was read. An `isinstance` test narrows an attribute only
    /// from here: a hand-off or an alias made before the test (`q = p`,
    /// `a = p.a`, `f = lambda: clear(p.a)`) lets code write the field after it.
    fn guard_intact(&self, chain: &str) -> bool {
        let (root, path) = chain.split_once('.').unwrap_or((chain, ""));
        self.untouched(root)
            && !self.exposed.contains(root)
            && self.defs(root).is_some_and(|d| d.iter().all(|(def, _)| *def == Def::Param))
            && (!path.contains('.') || (self.links_untouched(root) && !self.link_handed(chain)))
    }

    /// A walrus in `e` rebinds a name: what an `isinstance` test alone said
    /// about it, or about its attributes, is no longer known. (A test of
    /// another kind keeps its narrowing, as before this rule.)
    fn forget_rebound_isinstance_facts(&mut self, e: &Effects) {
        if e.rebinds.is_empty() {
            return;
        }
        let gone: Vec<String> = self
            .iso
            .iter()
            .filter(|n| e.rebinds.contains(n.split('.').next().unwrap_or(n)))
            .cloned()
            .collect();
        for name in gone {
            self.names.remove(&name);
            self.held.remove(&name);
            self.iso.remove(&name);
            if let Some(d) = self.defs.get_mut(&name) {
                d.iter_mut().for_each(|entry| entry.1 = false);
            }
        }
    }

    /// The state in which a statement or a test with effects `e` is evaluated:
    /// its operands are read against the state before it, so what `e` does to
    /// an `isinstance` narrowing in an operand that follows (a walrus in an
    /// argument, `clear(p)` in an earlier argument) is applied first. A held
    /// fact ends when `e` reaches its root, suspends or reads its `__dict__`,
    /// but not for a mere read of the chain (`handed`), which the evaluation
    /// of the statement's own use performs.
    pub fn forget_before(&mut self, e: &Effects) {
        let gone: Vec<String> = self
            .held
            .iter()
            .filter(|h| {
                let root = h.split('.').next().unwrap_or(h);
                e.suspends || e.reached.contains(root) || e.exposes.contains(root)
            })
            .cloned()
            .collect();
        for name in gone {
            self.names.remove(&name);
            self.held.remove(&name);
            self.iso.remove(&name);
        }
        self.forget_rebound_isinstance_facts(e);
    }

    /// The state after evaluating a node with effects `e`.
    pub fn apply(&mut self, e: &Effects) {
        self.names.retain(|n| e.keeps_member(n));
        for h in self.held.iter().filter(|h| e.hands_off(h)) {
            self.names.remove(h);
        }
        self.held.retain(|h| self.names.contains(h));
        self.forget_rebound_isinstance_facts(e);
        self.iso.retain(|h| self.names.contains(h));
        self.exposed.extend(e.exposes.iter().cloned());
        if e.suspends {
            self.untouched.clear();
            self.links_untouched.clear();
        } else {
            self.untouched.retain(|p| !e.reached.contains(p));
            self.links_untouched
                .retain(|p| !e.reached.contains(p) && !e.handed.contains(p));
            self.handed_paths.extend(e.handed_paths.iter().cloned());
        }
    }

    /// A rebound variable's attributes (`obj.f`, and `obj.fk.f` of a rebound
    /// `obj.fk`) no longer hold values assigned to them before
    /// (`Def::Param`), and are not narrowed.
    fn forget_attributes(&mut self, name: &str) {
        let prefix = format!("{name}.");
        if self.names.iter().any(|n| n.starts_with(&prefix)) {
            self.names.retain(|n| !n.starts_with(&prefix));
        }
        self.held.retain(|n| !n.starts_with(&prefix));
        self.iso.retain(|n| !n.starts_with(&prefix));
        for (n, defs) in self.defs.iter_mut() {
            if n.starts_with(&prefix) {
                *defs = vec![(Def::Param, false)];
            }
        }
    }

    /// Entry state: each of `params` holds its argument.
    pub fn with_params<'p>(params: impl IntoIterator<Item = &'p str>) -> Self {
        let mut n = Narrowed::new();
        for p in params {
            n.defs.insert(p.to_string(), vec![(Def::Param, false)]);
            n.untouched.insert(p.to_string());
            n.links_untouched.insert(p.to_string());
        }
        n
    }

    /// The state where control from any of `branches` meets.
    pub fn join(branches: &[Narrowed]) -> Option<Narrowed> {
        let (first, rest) = branches.split_first()?;
        let mut out = first.clone();
        for b in rest {
            out.names.retain(|n| b.names.contains(n));
            out.untouched.retain(|n| b.untouched.contains(n));
            out.links_untouched.retain(|n| b.links_untouched.contains(n));
            out.handed_paths.extend(b.handed_paths.iter().cloned());
            out.held.extend(b.held.iter().cloned());
            out.exposed.extend(b.exposed.iter().cloned());
            out.iso.extend(b.iso.iter().cloned());
            out.defs.retain(|name, defs| {
                let Some(other) = b.defs.get(name) else {
                    return false;
                };
                for &(d, nn) in other {
                    match defs.iter_mut().find(|(x, _)| *x == d) {
                        Some(entry) => entry.1 &= nn,
                        None => defs.push((d, nn)),
                    }
                }
                true
            });
        }
        out.held.retain(|h| out.names.contains(h));
        out.iso.retain(|h| out.names.contains(h));
        Some(out)
    }
}

/// Whether `link` (`p.a.b`) is a proper prefix of `chain` (`p.a.b.f`).
fn is_link_of(link: &str, chain: &str) -> bool {
    chain.len() > link.len() && chain.starts_with(link) && chain.as_bytes()[link.len()] == b'.'
}

/// A name known non-None when a test has some outcome (see `positive`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fact {
    pub name: String,
    /// Known by an `isinstance` test on an attribute (`p.f`, `p.a.f`), which
    /// holds only while a caller's guard on it would (`Narrowed::held`).
    pub held: bool,
    /// Known by an `isinstance` test, which a walrus in the same test can
    /// undo by rebinding the subject (`isinstance(x, str) and (x := y)`).
    by_isinstance: bool,
}

impl Fact {
    fn plain(name: impl Into<String>) -> Self {
        Fact { name: name.into(), held: false, by_isinstance: false }
    }
}

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
    exits: &Exits,
    v: &mut V,
) -> Narrowed {
    let mut start = narrowed.clone();
    if !exits.excludes_none.is_empty() {
        start.excludes_none = Rc::new(exits.excludes_none.clone());
    }
    walk(stmts, &start, exits, v, &mut HashMap::new())
}

/// `counts`: simple assignments seen so far per name (the next `Def::Assign` index).
fn walk<'a, V: FlowVisitor<'a>>(
    stmts: &'a [Stmt],
    narrowed: &Narrowed,
    exits: &Exits,
    v: &mut V,
    counts: &mut HashMap<String, usize>,
) -> Narrowed {
    let mut n = narrowed.clone();
    for stmt in stmts {
        match stmt {
            Stmt::If(s) => {
                n.forget_before(&effects(&s.test));
                v.header(&s.test, &n);
                n.extend(dereferenced(&s.test));
                let before = n.clone();
                n.apply(&effects(&s.test));
                let mut ends = Vec::new();
                let mut body_n = n.clone();
                body_n.assume(positive(&s.test, &before));
                let end = walk(&s.body, &body_n, exits, v, counts);
                if !always_exits(&s.body, exits) {
                    ends.push(end);
                }
                let mut negated = negative(&s.test, &before);
                let mut has_else = false;
                for clause in &s.elif_else_clauses {
                    let mut clause_n = n.clone();
                    clause_n.assume(negated.iter().cloned());
                    let end = match &clause.test {
                        Some(test) => {
                            let e = effects(test);
                            clause_n.forget_before(&e);
                            v.header(test, &clause_n);
                            n.apply(&e);
                            negated.retain(|x| x.survives(&e));
                            let before = clause_n.clone();
                            clause_n.apply(&e);
                            let mut bn = clause_n.clone();
                            bn.assume(positive(test, &before));
                            let end = walk(&clause.body, &bn, exits, v, counts);
                            negated.extend(negative(test, &before));
                            end
                        }
                        None => {
                            has_else = true;
                            walk(&clause.body, &clause_n, exits, v, counts)
                        }
                    };
                    if !always_exits(&clause.body, exits) {
                        ends.push(end);
                    }
                }
                if !has_else {
                    // No branch taken: every test was false.
                    let mut fall = n.clone();
                    fall.assume(negated);
                    ends.push(fall);
                }
                if let Some(joined) = Narrowed::join(&ends) {
                    n = joined;
                } else {
                    // Every branch exits: what follows is unreachable.
                    remove_all(&mut n, bound_names(std::slice::from_ref(stmt)));
                }
            }
            Stmt::Assert(a) => {
                n.forget_before(&effects_of_stmt(stmt));
                v.simple(stmt, &n);
                n.assume(positive(&a.test, &n));
                n.apply(&effects_of_stmt(stmt));
            }
            Stmt::For(f) => {
                n.forget_before(&effects(&f.iter));
                v.header(&f.iter, &n);
                n.extend(dereferenced(&f.iter));
                // A call in a later iteration may already have run.
                n.apply(&effects_of_stmt(stmt));
                remove_all(&mut n, bound_names(std::slice::from_ref(stmt)));
                walk(&f.body, &n, exits, v, counts);
                walk(&f.orelse, &n, exits, v, counts);
            }
            Stmt::While(w) => {
                remove_all(&mut n, bound_names(std::slice::from_ref(stmt)));
                // The test runs again after every iteration, so it sees what
                // the loop's own effects leave, not the state before the loop.
                n.apply(&effects_of_stmt(stmt));
                v.header(&w.test, &n);
                let mut body_n = n.clone();
                body_n.assume(positive(&w.test, &n));
                walk(&w.body, &body_n, exits, v, counts);
                walk(&w.orelse, &n, exits, v, counts);
            }
            Stmt::With(w) => {
                // The state that carries past the `with`: what the first item
                // narrowed holds, since nothing manages its expression. A later
                // item's expression runs inside the earlier managers
                // (`with suppress(AttributeError), x.open(): ...`), so what it
                // narrowed does not survive, like what the body narrowed.
                let mut after = n.clone();
                for (i, item) in w.items.iter().enumerate() {
                    n.forget_before(&effects(&item.context_expr));
                    v.header(&item.context_expr, &n);
                    n.extend(dereferenced(&item.context_expr));
                    n.apply(&effects(&item.context_expr));
                    if let Some(target) = &item.optional_vars {
                        remove_all(&mut n, bound_names_in_expr(target));
                        n.apply(&effects(target));
                    }
                    if i == 0 {
                        after = n.clone();
                    }
                }
                if w.is_async {
                    // `__aenter__` suspends before the body runs.
                    n.apply(&Effects::suspension());
                }
                // The manager may suppress an exception raised anywhere in the
                // body (`with suppress(AttributeError): x.upper()`), and then
                // control continues after any prefix of it. So what the body
                // narrowed does not carry past it, and its effects may have run.
                walk(&w.body, &n, exits, v, counts);
                n = after;
                remove_all(&mut n, bound_names(std::slice::from_ref(stmt)));
                n.apply(&effects_of_stmt(stmt));
            }
            Stmt::Try(t) => {
                let after = {
                    let mut a = n.clone();
                    remove_all(&mut a, bound_names(std::slice::from_ref(stmt)));
                    // A handler (and `finally`) can run after any prefix of the
                    // body, so a call there may already have removed a key.
                    // A handler or `else` that mutates and then exits never
                    // reaches the join, but a `finally` still runs after it.
                    a.apply(&effects_of_stmt(stmt));
                    a
                };
                let body_end = walk(&t.body, &n, exits, v, counts);
                let else_end = walk(&t.orelse, &body_end, exits, v, counts);
                let mut ends = Vec::new();
                if !always_exits(&t.body, exits) && !always_exits(&t.orelse, exits) {
                    ends.push(else_end);
                }
                for handler in &t.handlers {
                    let ast::ExceptHandler::ExceptHandler(h) = handler;
                    let end = walk(&h.body, &after, exits, v, counts);
                    if !always_exits(&h.body, exits) {
                        ends.push(end);
                    }
                }
                let joined = Narrowed::join(&ends).unwrap_or_else(|| after.clone());
                n = if t.finalbody.is_empty() {
                    joined
                } else {
                    // `finally` also runs on exceptional paths, from `after`.
                    let start = Narrowed::join(&[joined, after]).unwrap_or_default();
                    walk(&t.finalbody, &start, exits, v, counts)
                };
            }
            Stmt::Match(m) => {
                n.forget_before(&effects(&m.subject));
                v.header(&m.subject, &n);
                n.extend(dereferenced(&m.subject));
                // The cases run on a clone, so a call in a guard or a body
                // never reaches `n`: drop the facts for the whole statement.
                n.apply(&effects_of_stmt(stmt));
                remove_all(&mut n, bound_names(std::slice::from_ref(stmt)));
                for case in &m.cases {
                    walk(&case.body, &n, exits, v, counts);
                }
            }
            Stmt::FunctionDef(f) => {
                // Decorators and defaults run at the definition (the body
                // does not), but the body may capture a parameter's object.
                let mut v = Effects::new();
                f.decorator_list.iter().for_each(|d| v.visit_decorator(d));
                v.visit_parameters(&f.parameters);
                if let Some(r) = &f.returns {
                    v.visit_expr(r);
                }
                // The body runs when the function is called: the chains it hands
                // on, and those it writes, are as good as handed at the definition.
                let inner = effects_of_stmt(stmt);
                v.reached.extend(inner.reached);
                v.handed.extend(inner.handed);
                v.handed_paths.extend(inner.handed_paths);
                v.exposes.extend(inner.exposes);
                n.apply(&v);
                n.remove(f.name.as_str());
            }
            Stmt::ClassDef(c) => {
                // The class body, its bases and decorators run at the definition,
                // and its methods later, like a nested function's body.
                n.apply(&effects_of_stmt(stmt));
                n.remove(c.name.as_str());
            }
            _ => {
                n.forget_before(&effects_of_stmt(stmt));
                v.simple(stmt, &n);
                n.extend(stmt_dereferences(stmt));
                n.apply(&effects_of_stmt(stmt));
                let simple: Vec<String> = match stmt {
                    Stmt::Assign(a) => simple_assign_targets(a)
                        .or_else(|| simple_attr_targets(a))
                        .map(|pairs| pairs.into_iter().map(|(name, _)| name).collect())
                        .unwrap_or_default(),
                    Stmt::AnnAssign(a) if a.value.is_some() => match a.target.as_ref() {
                        Expr::Name(t) => vec![t.id.to_string()],
                        _ => Vec::new(),
                    },
                    _ => Vec::new(),
                };
                remove_all(&mut n, bound_names(std::slice::from_ref(stmt)));
                for name in simple {
                    let count = counts.entry(name.clone()).or_default();
                    n.rebind(&name, Def::Assign(*count));
                    *count += 1;
                }
                if let Stmt::AugAssign(a) = stmt {
                    let target = match a.target.as_ref() {
                        Expr::Name(t) => Some(t.id.to_string()),
                        other => attr_name(other),
                    };
                    if let Some(name) = target {
                        n.defs.insert(name, vec![(Def::Augmented, true)]);
                    }
                }
            }
        }
    }
    n
}

/// Names (and attributes of variables, as `obj.f`) that evaluating `expr`
/// certainly dereferences: `x.attr` (not a dunder, which `None` has),
/// `x.method(...)`, `x[i]`, `len(x)`. Evaluating them with `x` None raises,
/// so afterwards `x` is not None. Only unconditionally evaluated parts
/// count (the first operand of `and` / `or`, the test of `a if c else b`,
/// the first iterable of a comprehension; not lambda bodies).
pub fn dereferenced(expr: &Expr) -> Vec<String> {
    let mut out = Vec::new();
    deref_walk(expr, &mut out);
    out
}

fn deref_target(expr: &Expr, out: &mut Vec<String>) {
    match expr {
        Expr::Name(n) => out.push(n.id.to_string()),
        Expr::Attribute(_) => out.extend(attr_name(expr)),
        _ => {}
    }
}

fn deref_walk(expr: &Expr, out: &mut Vec<String>) {
    match expr {
        Expr::Attribute(a) => {
            let dunder = a.attr.starts_with("__") && a.attr.ends_with("__");
            if !dunder {
                deref_target(&a.value, out);
            }
            deref_walk(&a.value, out);
        }
        Expr::Subscript(s) => {
            deref_target(&s.value, out);
            deref_walk(&s.value, out);
            deref_walk(&s.slice, out);
        }
        Expr::Call(c) => {
            if let (Expr::Name(f), [arg]) = (c.func.as_ref(), &c.arguments.args[..]) {
                if f.id.as_str() == "len" && c.arguments.keywords.is_empty() {
                    deref_target(arg, out);
                }
            }
            deref_walk(&c.func, out);
            for arg in c.arguments.args.iter() {
                deref_walk(arg, out);
            }
            for kw in c.arguments.keywords.iter() {
                deref_walk(&kw.value, out);
            }
        }
        Expr::BoolOp(b) => {
            if let Some(first) = b.values.first() {
                deref_walk(first, out);
            }
        }
        Expr::If(i) => deref_walk(&i.test, out),
        Expr::Compare(c) => {
            deref_walk(&c.left, out);
            if let Some(first) = c.comparators.first() {
                deref_walk(first, out);
            }
        }
        Expr::ListComp(ast::ExprListComp { generators, .. })
        | Expr::SetComp(ast::ExprSetComp { generators, .. })
        | Expr::Generator(ast::ExprGenerator { generators, .. })
        | Expr::DictComp(ast::ExprDictComp { generators, .. }) => {
            if let Some(g) = generators.first() {
                deref_walk(&g.iter, out);
            }
        }
        Expr::Named(n) => deref_walk(&n.value, out),
        Expr::BinOp(b) => {
            deref_walk(&b.left, out);
            deref_walk(&b.right, out);
        }
        Expr::UnaryOp(u) => deref_walk(&u.operand, out),
        Expr::Await(a) => deref_walk(&a.value, out),
        Expr::Starred(s) => deref_walk(&s.value, out),
        Expr::Tuple(t) => t.elts.iter().for_each(|e| deref_walk(e, out)),
        Expr::List(l) => l.elts.iter().for_each(|e| deref_walk(e, out)),
        Expr::Set(s) => s.elts.iter().for_each(|e| deref_walk(e, out)),
        Expr::Dict(d) => {
            for item in d.items.iter() {
                if let Some(k) = &item.key {
                    deref_walk(k, out);
                }
                deref_walk(&item.value, out);
            }
        }
        Expr::Slice(s) => {
            for e in [&s.lower, &s.upper, &s.step].into_iter().flatten() {
                deref_walk(e, out);
            }
        }
        _ => {}
    }
}

/// What executing a simple statement certainly dereferences (see
/// `dereferenced`): its evaluated expressions and the objects of attribute
/// and subscript targets. `assert` does not count (it may be compiled out).
fn stmt_dereferences(stmt: &Stmt) -> Vec<String> {
    fn target(t: &Expr, out: &mut Vec<String>) {
        match t {
            Expr::Attribute(a) => {
                deref_target(&a.value, out);
                deref_walk(&a.value, out);
            }
            Expr::Subscript(s) => {
                deref_target(&s.value, out);
                deref_walk(&s.value, out);
                deref_walk(&s.slice, out);
            }
            Expr::Tuple(tt) => tt.elts.iter().for_each(|e| target(e, out)),
            Expr::List(l) => l.elts.iter().for_each(|e| target(e, out)),
            Expr::Starred(s) => target(&s.value, out),
            _ => {}
        }
    }
    let mut out = Vec::new();
    match stmt {
        Stmt::Expr(e) => deref_walk(&e.value, &mut out),
        Stmt::Assign(a) => {
            deref_walk(&a.value, &mut out);
            a.targets.iter().for_each(|t| target(t, &mut out));
        }
        Stmt::AnnAssign(a) => {
            if let Some(v) = &a.value {
                deref_walk(v, &mut out);
                target(&a.target, &mut out);
            }
        }
        Stmt::AugAssign(a) => {
            deref_walk(&a.value, &mut out);
            target(&a.target, &mut out);
        }
        Stmt::Delete(d) => d.targets.iter().for_each(|t| target(t, &mut out)),
        _ => {}
    }
    out
}

/// The fact `key in container` (see `positive`), kept among the narrowed
/// names: `[` cannot occur in an identifier, so it names no variable.
/// `container` is a name or `obj.f`; `key` a name or a rendered string literal.
pub fn member_key(container: &str, key: &str) -> String {
    format!("{container}[{key}]")
}

/// `member_key` of `key in container` expressions, when both have that form.
pub fn member_key_of(key: &Expr, container: &Expr) -> Option<String> {
    let container = match container {
        Expr::Name(n) => n.id.to_string(),
        other => attr_name(other)?,
    };
    let key = match key {
        Expr::Name(n) => n.id.to_string(),
        Expr::StringLiteral(s) => format!("{:?}", s.value.to_str()),
        _ => return None,
    };
    Some(member_key(&container, &key))
}

/// What evaluating a node may do to the facts the walk keeps.
#[derive(Debug, Default)]
pub struct Effects {
    /// It may remove a key from any dict: it has a call other than a `.get`
    /// on a name or `obj.f`, a `del`, a walrus (which can rebind the key) or
    /// a suspension.
    mutates: bool,
    /// The receivers of its `.get` calls. Only a dict's own `.get` is known
    /// not to mutate, so a fact about any other container is dropped.
    gets: HashSet<String>,
    /// Names whose objects it may hand to other code: a name read other than
    /// as `x.f` or in an `is` test (an argument, a receiver `x.m(...)`, a
    /// value stored or returned, an operand that runs `x`'s dunders).
    pub reached: HashSet<String>,
    /// Names `x` of which an attribute `x.a` is read as a value (an
    /// argument, a stored or returned value) rather than only as the
    /// receiver of a further attribute: the object `x.a` is then in other
    /// code's hands, which can write its fields. Also the roots of the chains
    /// it stores to or deletes other than by an assignment the walk tracks
    /// (`simple_attr_targets`, `x.f += v`): a comprehension or `for` target,
    /// or any store in a nested function or class body.
    pub handed: HashSet<String>,
    /// The chains `x.a.b` (two or more attributes deep) read as a value, each
    /// as `x.a.b`: unlike `x.a`, nothing here says whether the last
    /// attribute holds an object or a plain value, so only a guard on a longer
    /// chain through it is lost (`Narrowed::link_handed`), not every guard on `x`.
    pub handed_paths: HashSet<String>,
    /// The roots `x` of which a field may be written by code that no later
    /// node names: `x.__dict__` or `x.a.__dict__` read (the table can be
    /// indexed), a store to `x.f` in a nested function or class body or as a
    /// comprehension target, and a `nonlocal x` in a nested body. A store at
    /// the top level (a tuple, annotated, `for` or `with` target) is not
    /// here: `bound_names` forgets what it writes, as for `x.f = v`, which
    /// assumes that an attribute setter is not code that writes another field.
    pub exposes: HashSet<String>,
    /// The names a walrus in it binds.
    rebinds: HashSet<String>,
    /// It suspends (`await`, `yield`, `async for`, `async with`, an async
    /// comprehension): other code may then run on any object.
    suspends: bool,
    /// The parameters of each enclosing nested function or lambda: inside
    /// it, such a name is not the one the walk tracks.
    shadowed: Vec<HashSet<String>>,
    /// How many function or class bodies enclose the node.
    nested: usize,
}

impl Effects {
    fn new() -> Self {
        Self::default()
    }

    fn suspension() -> Self {
        Self::new().suspended()
    }

    fn suspended(mut self) -> Self {
        self.mutates = true;
        self.suspends = true;
        self
    }

    /// Whether a caller's guard on `chain` (`p.f`, `p.a.f`) ends at a node
    /// with these effects: code may run on any object, or is handed `p`, or,
    /// for a chain, an attribute of `p` or a proper prefix of the chain
    /// (`Scope::field_read_facts` reads the same conditions from the entry,
    /// through `Narrowed::untouched`, `links_untouched` and `link_handed`).
    pub fn hands_off(&self, chain: &str) -> bool {
        let (root, path) = chain.split_once('.').unwrap_or((chain, ""));
        self.suspends
            || self.reached.contains(root)
            || self.exposes.contains(root)
            || (path.contains('.')
                && (self.handed.contains(root) || self.handed_paths.iter().any(|h| is_link_of(h, chain))))
    }

    /// Whether the fact `n` (a name or a `member_key`) survives.
    pub fn keeps_member(&self, n: &str) -> bool {
        match n.split_once('[') {
            None => true,
            Some((container, _)) => !self.mutates && self.gets.iter().all(|g| g == container),
        }
    }

    /// A comprehension target `x.f` writes a field of `x` on each iteration,
    /// and no statement binds it for the walk (`bound_names`).
    fn expose_target(&mut self, target: &Expr) {
        match target {
            Expr::Attribute(a) => {
                let mut base = a.value.as_ref();
                while let Expr::Attribute(inner) = base {
                    base = inner.value.as_ref();
                }
                if let Expr::Name(n) = base {
                    if self.outer(n.id.as_str()) {
                        self.exposes.insert(n.id.to_string());
                    }
                }
            }
            Expr::Tuple(t) => t.elts.iter().for_each(|e| self.expose_target(e)),
            Expr::List(l) => l.elts.iter().for_each(|e| self.expose_target(e)),
            Expr::Starred(st) => self.expose_target(&st.value),
            _ => {}
        }
    }

    fn reach_root(&mut self, expr: &Expr) {
        match expr {
            Expr::Name(n) if self.outer(n.id.as_str()) => {
                self.reached.insert(n.id.to_string());
            }
            Expr::Attribute(a) => self.reach_root(&a.value),
            _ => {}
        }
    }

    /// Whether `name` here is the walked function's own, not a parameter of
    /// a nested function or lambda.
    fn outer(&self, name: &str) -> bool {
        !self.shadowed.iter().any(|s| s.contains(name))
    }

    /// Visit a body that runs later, as a function of `params`.
    fn visit_nested(&mut self, params: HashSet<String>, visit: impl FnOnce(&mut Self)) {
        self.shadowed.push(params);
        self.nested += 1;
        visit(self);
        self.nested -= 1;
        self.shadowed.pop();
    }
}

impl<'a> Visitor<'a> for Effects {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::Delete(d) => {
                self.mutates = true;
                d.targets.iter().for_each(|t| self.reach_root(t));
            }
            // A nested body that rebinds a name of this function can run later.
            Stmt::Nonlocal(nl) if self.nested > 0 => {
                nl.names.iter().for_each(|n| {
                    self.exposes.insert(n.id.to_string());
                });
            }
            Stmt::For(f) if f.is_async => *self = std::mem::take(self).suspended(),
            Stmt::With(w) if w.is_async => *self = std::mem::take(self).suspended(),
            // The walk tracks these stores itself.
            Stmt::Assign(a) if self.nested == 0 && simple_attr_targets(a).is_some() => {
                return self.visit_expr(&a.value);
            }
            Stmt::AugAssign(a) if self.nested == 0 && attr_name(&a.target).is_some() => {
                return self.visit_expr(&a.value);
            }
            // Decorators, defaults and annotations run at the definition, in this scope.
            Stmt::FunctionDef(f) => {
                f.decorator_list.iter().for_each(|d| self.visit_decorator(d));
                if let Some(t) = &f.type_params {
                    self.visit_type_params(t);
                }
                self.visit_parameters(&f.parameters);
                if let Some(r) = &f.returns {
                    self.visit_annotation(r);
                }
                let params = f.parameters.iter().map(|p| p.name().id.to_string()).collect();
                return self.visit_nested(params, |v| v.visit_body(&f.body));
            }
            Stmt::ClassDef(c) => {
                c.decorator_list.iter().for_each(|d| self.visit_decorator(d));
                if let Some(t) = &c.type_params {
                    self.visit_type_params(t);
                }
                if let Some(a) = &c.arguments {
                    self.visit_arguments(a);
                }
                return self.visit_nested(HashSet::new(), |v| v.visit_body(&c.body));
            }
            _ => {}
        }
        visitor::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Call(c) => {
                if let Expr::Attribute(a) = c.func.as_ref() {
                    self.reach_root(&a.value);
                }
                // `super()` hands the method's own object to the parent's code, by a
                // method call, a property read or a stored `s = super()`. The walk
                // does not know which parameter that is, so it counts as other code
                // running on any object.
                if matches!(c.func.as_ref(), Expr::Name(n) if n.id.as_str() == "super") {
                    *self = std::mem::take(self).suspended();
                }
                let get_on = match c.func.as_ref() {
                    Expr::Attribute(a) if a.attr.as_str() == "get" => match a.value.as_ref() {
                        Expr::Name(n) => Some(n.id.to_string()),
                        other => attr_name(other),
                    },
                    _ => None,
                };
                match get_on {
                    Some(r) => {
                        self.gets.insert(r);
                    }
                    None => self.mutates = true,
                }
            }
            Expr::Name(n) if matches!(n.ctx, ExprContext::Load) => {
                if self.outer(n.id.as_str()) {
                    self.reached.insert(n.id.to_string());
                }
            }
            Expr::Lambda(l) => {
                let mut params = HashSet::new();
                if let Some(p) = &l.parameters {
                    self.visit_parameters(p);
                    params = p.iter().map(|p| p.name().id.to_string()).collect();
                }
                return self.visit_nested(params, |v| v.visit_expr(&l.body));
            }
            // `x.f` reads a field of `x` and hands `x` to nothing, but it hands
            // the object `x.f` to whatever uses it as a value.
            Expr::Attribute(a) => {
                if a.attr.as_str() == "__dict__" {
                    let mut base = a.value.as_ref();
                    while let Expr::Attribute(inner) = base {
                        base = inner.value.as_ref();
                    }
                    if let Expr::Name(n) = base {
                        if self.outer(n.id.as_str()) {
                            self.exposes.insert(n.id.to_string());
                        }
                    }
                }
                if matches!(a.ctx, ExprContext::Store | ExprContext::Del) {
                    let mut base = a.value.as_ref();
                    while let Expr::Attribute(inner) = base {
                        base = inner.value.as_ref();
                    }
                    if let Expr::Name(n) = base {
                        if self.outer(n.id.as_str()) {
                            self.handed.insert(n.id.to_string());
                            // A store in a nested body writes a field of the
                            // root when the body runs, which no node names.
                            if self.nested > 0 {
                                self.exposes.insert(n.id.to_string());
                            }
                        }
                    }
                }
                if let Expr::Name(n) = a.value.as_ref() {
                    if matches!(a.ctx, ExprContext::Load) && self.outer(n.id.as_str()) {
                        self.handed.insert(n.id.to_string());
                    }
                    return;
                }
                // `x.a.f` reads a field of `x.a`: `x.a` is only the receiver. The
                // object `x.a.f` is in other code's hands, though: when `x.a.f` is
                // itself a link, code that holds it can write the fields of the
                // chains through it.
                if matches!(a.ctx, ExprContext::Load) {
                    if let Some(path) = attr_name(expr) {
                        if path.split('.').next().is_some_and(|root| self.outer(root)) {
                            self.handed_paths.insert(path);
                        }
                    }
                }
                let mut base = a.value.as_ref();
                while let Expr::Attribute(inner) = base {
                    if matches!(inner.value.as_ref(), Expr::Name(_)) {
                        return;
                    }
                    base = inner.value.as_ref();
                }
                self.visit_expr(base);
                return;
            }
            // An identity test runs no code of its operands.
            Expr::Compare(c) if c.ops.iter().all(|op| matches!(op, CmpOp::Is | CmpOp::IsNot)) => {
                for e in std::iter::once(c.left.as_ref()).chain(c.comparators.iter()) {
                    // `x` and `x.a.b` run no code and hand nothing over; the base
                    // of any other chain is evaluated.
                    let mut base = e;
                    while let Expr::Attribute(a) = base {
                        base = a.value.as_ref();
                    }
                    if !matches!(base, Expr::Name(_)) {
                        self.visit_expr(base);
                    }
                }
                return;
            }
            Expr::Named(named) => {
                self.mutates = true;
                if let Expr::Name(target) = named.target.as_ref() {
                    if self.outer(target.id.as_str()) {
                        self.rebinds.insert(target.id.to_string());
                    }
                }
            }
            // A suspension hands control to code that may mutate any object.
            Expr::Await(_) | Expr::Yield(_) | Expr::YieldFrom(_) => {
                *self = std::mem::take(self).suspended();
            }
            Expr::ListComp(ast::ExprListComp { generators, .. })
            | Expr::SetComp(ast::ExprSetComp { generators, .. })
            | Expr::Generator(ast::ExprGenerator { generators, .. })
            | Expr::DictComp(ast::ExprDictComp { generators, .. }) => {
                if generators.iter().any(|g| g.is_async) {
                    *self = std::mem::take(self).suspended();
                }
                generators.iter().for_each(|g| self.expose_target(&g.target));
            }
            _ => {}
        }
        visitor::walk_expr(self, expr);
    }
}

/// The effects of evaluating `expr`.
pub fn effects(expr: &Expr) -> Effects {
    let mut v = Effects::new();
    v.visit_expr(expr);
    v
}

/// The effects of running `stmt` (with every nested block).
pub fn effects_of_stmt(stmt: &Stmt) -> Effects {
    let mut v = Effects::new();
    v.visit_stmt(stmt);
    v
}

fn remove_all(n: &mut Narrowed, names: HashSet<String>) {
    for name in names {
        n.remove(&name);
    }
}

/// Names known non-None when `test` is true (attributes of variables as
/// `obj.f`, see `attr_name`), and `member_key` facts. `n` is the state
/// before `test` is evaluated (its own effects not applied), which knows the
/// calls that rule out None and what was handed off earlier.
pub fn positive(test: &Expr, n: &Narrowed) -> Vec<Fact> {
    let mut out: Vec<Fact> = match test {
        Expr::Name(n) => vec![Fact::plain(n.id.as_str())],
        Expr::Attribute(_) => attr_name(test).into_iter().map(Fact::plain).collect(),
        Expr::Compare(c) => none_comparison(c, CmpOp::IsNot)
            .into_iter()
            .chain(membership(c, CmpOp::In))
            .map(Fact::plain)
            .collect(),
        // `isinstance(x, str).__eq__(y)` starts at the same offset: only the
        // call of the name itself is the one resolution marked.
        Expr::Call(c)
            if matches!(c.func.as_ref(), Expr::Name(f) if f.id.as_str() == "isinstance")
                && n.excludes_none.contains(&c.start().to_u32()) =>
        {
            match c.arguments.args.first() {
                Some(Expr::Name(s)) => vec![Fact { by_isinstance: true, ..Fact::plain(s.id.as_str()) }],
                // An attribute is narrowed only where a caller's guard on it
                // would hold since the entry; `n` is the state before the test.
                Some(s) => attr_name(s)
                    .filter(|name| n.guard_intact(name))
                    .map(|name| Fact { name, held: true, by_isinstance: true })
                    .into_iter()
                    .collect(),
                None => Vec::new(),
            }
        }
        Expr::BoolOp(b) if matches!(b.op, BoolOp::And) => {
            b.values.iter().flat_map(|v| positive(v, n)).collect()
        }
        Expr::UnaryOp(u) if matches!(u.op, UnaryOp::Not) => negative(&u.operand, n),
        _ => Vec::new(),
    };
    out.retain(|f| f.survives(&effects(test)));
    out
}

/// Names known non-None when `test` is false, and `member_key` facts. A
/// false `isinstance` says nothing about None.
pub fn negative(test: &Expr, n: &Narrowed) -> Vec<Fact> {
    let mut out: Vec<Fact> = match test {
        Expr::Compare(c) => none_comparison(c, CmpOp::Is)
            .into_iter()
            .chain(membership(c, CmpOp::NotIn))
            .map(Fact::plain)
            .collect(),
        Expr::BoolOp(b) if matches!(b.op, BoolOp::Or) => {
            b.values.iter().flat_map(|v| negative(v, n)).collect()
        }
        Expr::UnaryOp(u) if matches!(u.op, UnaryOp::Not) => positive(&u.operand, n),
        _ => Vec::new(),
    };
    out.retain(|f| f.survives(&effects(test)));
    out
}

impl Fact {
    /// Whether the fact holds after a node with effects `e`, such as the
    /// rest of its own test: a call may remove a key again, or hand off what
    /// a held fact is about.
    fn survives(&self, e: &Effects) -> bool {
        let root = self.name.split('.').next().unwrap_or(&self.name);
        e.keeps_member(&self.name)
            && !(self.held && e.hands_off(&self.name))
            && !(self.by_isinstance && e.rebinds.contains(root))
    }
}

/// `x <op> <display of non-None literals>`: `x`, which equals one of them.
/// `k <op> d`: the `member_key` of `k in d`.
fn membership(c: &ast::ExprCompare, op: CmpOp) -> Option<String> {
    if c.ops.len() != 1 || c.ops[0] != op {
        return None;
    }
    let elts = match &c.comparators[0] {
        Expr::Set(s) => &s.elts,
        Expr::List(l) => &l.elts,
        Expr::Tuple(t) => &t.elts,
        container => return member_key_of(&c.left, container),
    };
    let literal = |e: &Expr| {
        matches!(
            e,
            Expr::StringLiteral(_) | Expr::BytesLiteral(_) | Expr::NumberLiteral(_) | Expr::BooleanLiteral(_)
        )
    };
    if elts.is_empty() || !elts.iter().all(literal) {
        return None;
    }
    match c.left.as_ref() {
        Expr::Name(n) => Some(n.id.to_string()),
        left => attr_name(left),
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
        (e @ Expr::Attribute(_), Expr::NoneLiteral(_))
        | (Expr::NoneLiteral(_), e @ Expr::Attribute(_)) => attr_name(e),
        _ => None,
    }
}

/// `obj.f` (or `obj.fk.f`, any depth) for an attribute chain rooted at a
/// variable: the flow name of the attribute (tracked like a local: `obj.f = v`
/// rebinds it, `if obj.f is None:` narrows it).
pub fn attr_name(expr: &Expr) -> Option<String> {
    let Expr::Attribute(a) = expr else { return None };
    let receiver = match a.value.as_ref() {
        Expr::Name(n) => n.id.to_string(),
        inner => attr_name(inner)?,
    };
    Some(format!("{receiver}.{}", a.attr))
}

/// `(obj.f, value)` pairs of an assignment whose every target is an
/// attribute of a variable (`self.f = v`, `a.x = b.y = v`); `None` otherwise.
pub fn simple_attr_targets(a: &ast::StmtAssign) -> Option<Vec<(String, &Expr)>> {
    a.targets
        .iter()
        .map(|t| attr_name(t).map(|n| (n, a.value.as_ref())))
        .collect()
}

/// Whether control never falls off the end of `body` (it ends in
/// `return` / `raise` / `continue` / `break`, a call in `exits`, or an `if`
/// or exhaustive `match` whose every branch does).
pub fn always_exits(body: &[Stmt], exits: &Exits) -> bool {
    match body.last() {
        Some(Stmt::Return(_) | Stmt::Raise(_) | Stmt::Continue(_) | Stmt::Break(_)) => true,
        Some(s @ Stmt::Expr(_)) => exits.is_exit_call(s),
        Some(Stmt::If(s)) => {
            always_exits(&s.body, exits)
                && s.elif_else_clauses.iter().any(|c| c.test.is_none())
                && s.elif_else_clauses.iter().all(|c| always_exits(&c.body, exits))
        }
        Some(Stmt::Match(m)) => {
            exits.is_exhaustive(m) && m.cases.iter().all(|c| always_exits(&c.body, exits))
        }
        _ => false,
    }
}

/// Whether some case of a `match` always matches: an unguarded wildcard
/// (`case _:`), capture (`case x:`), or an or-pattern containing one.
pub fn match_is_exhaustive(m: &ast::StmtMatch) -> bool {
    fn irrefutable(p: &Pattern) -> bool {
        match p {
            Pattern::MatchAs(a) => a.pattern.as_deref().is_none_or(irrefutable),
            Pattern::MatchOr(o) => o.patterns.iter().any(irrefutable),
            _ => false,
        }
    }
    m.cases
        .iter()
        .any(|c| c.guard.is_none() && irrefutable(&c.pattern))
}

/// Whether execution can reach the end of a function body (an implicit `return None`).
pub fn falls_through(body: &[Stmt], exits: &Exits) -> bool {
    !terminates(body, exits, Raised::Propagates)
}

/// What happens to an exception raised in the body being judged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Raised {
    /// It leaves the function (unless a `try` there catches it).
    Propagates,
    /// It reaches a context manager, whose `__exit__` may suppress it
    /// (`contextlib.suppress`, `pytest.raises`), and then control continues
    /// after the `with`.
    MayBeSuppressed,
}

fn terminates(body: &[Stmt], exits: &Exits, raised: Raised) -> bool {
    let terminates = |b: &[Stmt]| terminates(b, exits, raised);
    let propagates = raised == Raised::Propagates;
    match body.last() {
        Some(Stmt::Return(_)) => true,
        Some(Stmt::Raise(_)) => propagates,
        Some(s @ Stmt::Expr(_)) => exits.is_exit_call(s),
        Some(Stmt::If(s)) => {
            terminates(&s.body)
                && s.elif_else_clauses.iter().any(|c| c.test.is_none())
                && s.elif_else_clauses.iter().all(|c| terminates(&c.body))
        }
        // Without a `break`, the loop ends only by `return` or an exception.
        Some(Stmt::While(w)) => {
            propagates
                && matches!(w.test.as_ref(), Expr::BooleanLiteral(b) if b.value)
                && !contains_break(&w.body)
        }
        // Only a `return` ends a `with` body whatever the manager does. A
        // statement that raises before it is not yet reached (see the module
        // documentation).
        Some(Stmt::With(w)) => {
            crate::flow::terminates(&w.body, &Exits::default(), Raised::MayBeSuppressed)
        }
        Some(Stmt::Match(m)) => {
            exits.is_exhaustive(m) && m.cases.iter().all(|c| terminates(&c.body))
        }
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
        skip_aug: false,
        skip_for: false,
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
        skip_aug: false,
        skip_for: false,
    };
    c.visit_expr(expr);
    c.names
}

/// Every name read in `expr` (at any depth, lambdas and comprehensions included).
pub fn names_in_expr(expr: &Expr) -> HashSet<String> {
    struct N(HashSet<String>);
    impl<'a> Visitor<'a> for N {
        fn visit_expr(&mut self, expr: &'a Expr) {
            if let Expr::Name(n) = expr {
                self.0.insert(n.id.to_string());
            }
            visitor::walk_expr(self, expr);
        }
    }
    let mut n = N(HashSet::new());
    n.visit_expr(expr);
    n.0
}

struct Binders {
    names: HashSet<String>,
    /// Names bound by `import` statements.
    imports: HashSet<String>,
    /// Skip targets of simple `name = value` assignments (tracked separately).
    skip_simple: bool,
    /// Skip `name op= value` targets.
    skip_aug: bool,
    /// Skip `for name in ...` targets (a plain name).
    skip_for: bool,
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
            Stmt::Assign(a)
                if self.skip_simple
                    && (simple_assign_targets(a).is_some() || simple_attr_targets(a).is_some()) =>
            {
                self.visit_expr(&a.value);
            }
            Stmt::For(f) if self.skip_for && matches!(f.target.as_ref(), Expr::Name(_)) => {
                self.visit_expr(&f.iter);
                for s in f.body.iter().chain(f.orelse.iter()) {
                    self.visit_stmt(s);
                }
            }
            Stmt::AugAssign(a) if self.skip_aug && matches!(a.target.as_ref(), Expr::Name(_)) => {
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
            Expr::Attribute(a) if matches!(a.ctx, ExprContext::Store | ExprContext::Del) => {
                // `obj.f` stored other than by a simple assignment.
                self.names.extend(attr_name(expr));
                visitor::walk_expr(self, expr);
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
    /// The annotation of an annotated assignment (`d: dict[str, str] = ...`).
    pub annotation: Option<&'a Expr>,
}

/// Flow-insensitive facts about a function body.
#[derive(Debug, Default)]
pub struct FunctionFlow<'a> {
    /// `name = value` assignments per local name.
    pub assignments: HashMap<String, Vec<Assignment<'a>>>,
    /// `obj.f = value` assignments per attribute (`simple_attr_targets`),
    /// indexed like `assignments` (`Def::Assign(i)` of the name `obj.f`).
    pub attr_assignments: HashMap<String, Vec<Assignment<'a>>>,
    /// Names bound in any other way; their values are unknown.
    pub opaque: HashSet<String>,
    /// Names of `opaque` bound other than by simple assignment only by
    /// augmented assignment (`total += x`): such a value is never `None`.
    pub augmented_only: HashSet<String>,
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
    /// Escaping locals whose every escaping use is as a direct call
    /// argument: the calls and positions (a dict passed to a function that
    /// only splats it is not mutated).
    pub passed: HashMap<String, Vec<(&'a ast::ExprCall, ArgPos)>>,
    /// Names bound only by `for name in <iterable>` loops, with the iterables.
    pub loop_vars: HashMap<String, Vec<&'a Expr>>,
    /// The state at the start of the body (walks of the body start here).
    pub entry: Narrowed,
    /// The body contains `yield` / `yield from` (outside nested definitions):
    /// calling the function returns a generator, never `None`.
    pub is_generator: bool,
    /// The statements of the body that never complete (walks of the body use them).
    pub exits: Exits,
}

/// The `obj.f` names of simple attribute assignments in `body` (not in
/// nested definitions).
fn assigned_attributes(body: &[Stmt]) -> HashSet<String> {
    struct A(HashSet<String>);
    impl<'a> Visitor<'a> for A {
        fn visit_stmt(&mut self, stmt: &'a Stmt) {
            match stmt {
                Stmt::FunctionDef(_) | Stmt::ClassDef(_) => {}
                Stmt::Assign(a) => {
                    if let Some(pairs) = simple_attr_targets(a) {
                        self.0.extend(pairs.into_iter().map(|(n, _)| n));
                    }
                }
                _ => visitor::walk_stmt(self, stmt),
            }
        }
    }
    let mut a = A(HashSet::new());
    for s in body {
        a.visit_stmt(s);
    }
    a.0
}

/// Whether `body` yields (not counting nested functions, classes and lambdas).
pub fn is_generator_body(body: &[Stmt]) -> bool {
    struct Y(bool);
    impl<'a> Visitor<'a> for Y {
        fn visit_stmt(&mut self, stmt: &'a Stmt) {
            if !matches!(stmt, Stmt::FunctionDef(_) | Stmt::ClassDef(_)) {
                visitor::walk_stmt(self, stmt);
            }
        }
        fn visit_expr(&mut self, expr: &'a Expr) {
            match expr {
                Expr::Yield(_) | Expr::YieldFrom(_) => self.0 = true,
                Expr::Lambda(_) => {}
                _ => visitor::walk_expr(self, expr),
            }
        }
    }
    let mut y = Y(false);
    for s in body {
        y.visit_stmt(s);
    }
    y.0
}

impl<'a> FunctionFlow<'a> {
    pub fn of(body: &'a [Stmt]) -> Self {
        Self::with_entry(body, Narrowed::new(), Exits::default())
    }

    /// The flow of an extracted function's body.
    pub fn of_info(f: &'a crate::function_extractor::FunctionInfo) -> Self {
        let params = Narrowed::with_params(f.params.iter().map(|p| p.name.as_str()));
        Self::with_entry(&f.body, params, f.exits.clone())
    }

    /// The flow of a function body whose parameters are `params`.
    pub fn of_function<'p>(body: &'a [Stmt], params: impl IntoIterator<Item = &'p str>) -> Self {
        Self::with_entry(body, Narrowed::with_params(params), Exits::default())
    }

    fn with_entry(body: &'a [Stmt], mut entry: Narrowed, exits: Exits) -> Self {
        // Every attribute the body assigns starts with its value from before.
        for name in assigned_attributes(body) {
            entry.defs.insert(name, vec![(Def::Param, false)]);
        }
        let mut flow = FunctionFlow {
            falls_through: falls_through(body, &exits),
            is_generator: is_generator_body(body),
            entry: entry.clone(),
            ..Default::default()
        };
        let mut binders = Binders {
            names: HashSet::new(),
            imports: HashSet::new(),
            skip_simple: true,
            skip_aug: false,
            skip_for: false,
        };
        let mut non_aug = Binders {
            names: HashSet::new(),
            imports: HashSet::new(),
            skip_simple: true,
            skip_aug: true,
            skip_for: false,
        };
        for s in body {
            binders.visit_stmt(s);
            non_aug.visit_stmt(s);
        }
        flow.augmented_only = binders
            .names
            .difference(&non_aug.names)
            .filter(|n| !non_aug.imports.contains(*n))
            .cloned()
            .collect();
        flow.imported = binders
            .imports
            .difference(&binders.names)
            .cloned()
            .collect();
        // Names bound only as `for name in <iterable>` targets.
        let mut non_for = Binders {
            names: HashSet::new(),
            imports: HashSet::new(),
            skip_simple: false,
            skip_aug: false,
            skip_for: true,
        };
        let mut loops = ForLoops(HashMap::new());
        for s in body {
            non_for.visit_stmt(s);
            loops.visit_stmt(s);
        }
        flow.loop_vars = loops
            .0
            .into_iter()
            .filter(|(n, _)| !non_for.names.contains(n) && !non_for.imports.contains(n))
            .collect();
        flow.opaque = binders.names;
        flow.opaque.extend(binders.imports);
        walk_block(body, &entry, &exits, &mut flow);
        flow.exits = exits;
        let mut uses = Uses {
            escaping: HashSet::new(),
            passes: HashMap::new(),
            other: HashSet::new(),
        };
        for s in body {
            uses.visit_stmt(s);
        }
        flow.escaping = uses.escaping;
        flow.passed = uses.passes;
        flow.passed.retain(|name, _| !uses.other.contains(name));
        let mut shared = SharedNames::default();
        for s in body {
            shared.visit_stmt(s);
        }
        flow.opaque.extend(shared.names.iter().cloned());
        flow.augmented_only.retain(|n| !shared.names.contains(n));
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
                            annotation: None,
                        });
                    }
                } else if let Some(pairs) = simple_attr_targets(a) {
                    for (name, value) in pairs {
                        self.attr_assignments.entry(name).or_default().push(Assignment {
                            value,
                            narrowed: narrowed.clone(),
                            annotation: None,
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
                            annotation: Some(&a.annotation),
                        });
                }
            }
            Stmt::Return(r) => self.returns.push((r.value.as_deref(), narrowed.clone())),
            _ => {}
        }
    }

    fn header(&mut self, _expr: &'a Expr, _narrowed: &Narrowed) {}
}

/// `for name in <iterable>` loops (not in nested definitions): name -> iterables.
struct ForLoops<'a>(HashMap<String, Vec<&'a Expr>>);

impl<'a> Visitor<'a> for ForLoops<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) => {}
            Stmt::For(f) => {
                if let Expr::Name(n) = f.target.as_ref() {
                    self.0.entry(n.id.to_string()).or_default().push(&f.iter);
                }
                visitor::walk_stmt(self, stmt);
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }
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

/// Where a local is passed directly as a call argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgPos {
    /// The `i`-th positional argument.
    Positional(usize),
    /// The keyword argument `name=`.
    Keyword(String),
}

/// Collects names used in ways that may mutate or alias a dict.
struct Uses<'a> {
    escaping: HashSet<String>,
    /// Names passed directly as call arguments, with the calls.
    passes: HashMap<String, Vec<(&'a ast::ExprCall, ArgPos)>>,
    /// Names that escape other than by being passed directly as an argument.
    other: HashSet<String>,
}

const READ_ONLY_DICT_METHODS: [&str; 5] = ["get", "keys", "values", "items", "copy"];

impl<'a> Uses<'a> {
    /// `arg` is an argument of `call` at `pos`.
    fn visit_arg(&mut self, call: &'a ast::ExprCall, arg: &'a Expr, pos: ArgPos) {
        match arg {
            Expr::Name(n) if matches!(n.ctx, ExprContext::Load) => {
                self.escaping.insert(n.id.to_string());
                self.passes.entry(n.id.to_string()).or_default().push((call, pos));
            }
            _ => self.visit_expr(arg),
        }
    }
}

impl<'a> Visitor<'a> for Uses<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Name(n) if matches!(n.ctx, ExprContext::Load) => {
                self.escaping.insert(n.id.to_string());
                self.other.insert(n.id.to_string());
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
                let mut positional = true;
                for (i, arg) in call.arguments.args.iter().enumerate() {
                    positional &= !matches!(arg, Expr::Starred(_));
                    if positional {
                        self.visit_arg(call, arg, ArgPos::Positional(i));
                    } else {
                        self.visit_expr(arg);
                    }
                }
                for kw in call.arguments.keywords.iter() {
                    match &kw.arg {
                        // `f(**name)` reads the dict.
                        None if matches!(kw.value, Expr::Name(_)) => continue,
                        None => self.visit_expr(&kw.value),
                        Some(k) => self.visit_arg(call, &kw.value, ArgPos::Keyword(k.to_string())),
                    }
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
                ruff_python_ast::Mod::Module(m) => m.body.to_vec(),
                _ => unreachable!(),
            };
        match stmts.into_iter().next() {
            Some(Stmt::FunctionDef(f)) => f.body.to_vec(),
            _ => panic!("expected a function"),
        }
    }

    /// Narrowing in effect at each `use(...)` expression statement, in order.
    fn narrowing_at_uses(src: &str) -> Vec<Vec<String>> {
        narrowing_with_exits(src, &Exits::default())
    }

    fn narrowing_with_exits(src: &str, exits: &Exits) -> Vec<Vec<String>> {
        struct V(Vec<Vec<String>>);
        impl<'a> FlowVisitor<'a> for V {
            fn simple(&mut self, stmt: &'a Stmt, n: &Narrowed) {
                if let Stmt::Expr(e) = stmt {
                    if let Expr::Call(c) = e.value.as_ref() {
                        // `use()` observes the state; `probe.get()` does too and, being
                        // a `.get` call, leaves member facts alone.
                        let observed = match c.func.as_ref() {
                            Expr::Name(f) => f.id.as_str() == "use",
                            Expr::Attribute(a) => {
                                a.attr.as_str() == "get"
                                    && matches!(a.value.as_ref(), Expr::Name(v) if v.id.as_str() == "probe")
                            }
                            _ => false,
                        };
                        if observed {
                            let mut names: Vec<String> = n.names().cloned().collect();
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
        // The entry state: each parameter of the first `def` holds its argument.
        let params = src
            .strip_prefix("def ")
            .and_then(|rest| rest.split_once('(')?.1.split_once(')'))
            .map(|(list, _)| list.split(',').map(str::trim).filter(|p| !p.is_empty()).collect::<Vec<_>>())
            .unwrap_or_default();
        walk_block(&b, &Narrowed::with_params(params), exits, &mut v);
        v.0
    }

    /// `Exits` marking the statements of `src` that start with `calls` / `matches`.
    fn exits_at(src: &str, calls: &[&str], matches: &[&str]) -> Exits {
        let at = |text: &&str| src.find(text).expect("statement") as u32;
        Exits {
            calls: calls.iter().map(at).collect(),
            matches: matches.iter().map(at).collect(),
            ..Exits::default()
        }
    }

    /// Narrowing at each `use()` with every `isinstance(` call of `src` in
    /// `Exits::excludes_none`, as `exits::function_exits` marks the builtin.
    fn narrowing_with_isinstance(src: &str) -> Vec<Vec<String>> {
        let exits = Exits {
            excludes_none: src.match_indices("isinstance(").map(|(at, _)| at as u32).collect(),
            ..Exits::default()
        };
        narrowing_with_exits(src, &exits)
    }

    #[test]
    fn test_attribute_chains_narrow_and_rebinding_a_link_forgets_them() {
        let src = "def f(o, c):\n    if o.fk.x is None:\n        return\n    use()\n    o.fk = c\n    use()\n    if o.fk.x:\n        use()\n    o = c\n    use()\n";
        let names = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            narrowing_at_uses(src),
            vec![names(&["o", "o.fk", "o.fk.x"]), names(&["o"]), names(&["o", "o.fk", "o.fk.x"]), names(&[])]
        );
    }

    #[test]
    fn test_rebinding_a_middle_link_forgets_its_chains_and_no_others() {
        let src = "def f(o, c):\n    if o.a.b.x is None:\n        return\n    if o.ab.x is None:\n        return\n    if o.other.x is None:\n        return\n    use()\n    o.a = c\n    use()\n";
        let names = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            narrowing_at_uses(src),
            vec![
                names(&["o", "o.a", "o.a.b", "o.a.b.x", "o.ab", "o.ab.x", "o.other", "o.other.x"]),
                names(&["o", "o.ab", "o.ab.x", "o.other", "o.other.x"]),
            ]
        );
    }

    #[test]
    fn test_exit_calls_end_the_flow() {
        let src = "def f(x):\n    if x is None:\n        stop(1)\n    use()\n    stop(2)\n";
        let exits = exits_at(src, &["stop(1)", "stop(2)"], &[]);
        assert_eq!(narrowing_with_exits(src, &exits), vec![vec!["x".to_string()]]);
        assert!(!falls_through(&body(src), &exits));
        assert_eq!(narrowing_at_uses(src), vec![Vec::<String>::new()]);
        assert!(falls_through(&body(src), &Exits::default()));
        // Only an expression statement is a call exit.
        let src = "def f(x):\n    y = stop(2)\n";
        assert!(falls_through(&body(src), &exits_at(src, &["y = stop(2)"], &[])));
    }

    #[test]
    fn test_marked_match_is_exhaustive() {
        let src = "def f(c, x):\n    if x is None:\n        match c:\n            case E.A:\n                return 1\n    use()\n    match c:\n        case E.A:\n            return 1\n";
        let exits = exits_at(src, &[], &["match c:\n            case", "match c:\n        case"]);
        assert_eq!(narrowing_with_exits(src, &exits), vec![vec!["x".to_string()]]);
        assert!(!falls_through(&body(src), &exits));
        assert_eq!(narrowing_at_uses(src), vec![Vec::<String>::new()]);
        assert!(falls_through(&body(src), &Exits::default()));
    }

    #[test]
    fn test_marked_match_with_a_case_that_falls_through_is_no_exit() {
        let src = "def f(c, x):\n    if x is None:\n        match c:\n            case E.A:\n                pass\n    use()\n";
        let exits = exits_at(src, &[], &["match c:"]);
        assert_eq!(narrowing_with_exits(src, &exits), vec![Vec::<String>::new()]);
        let src = "def f(c):\n    match c:\n        case E.A:\n            pass\n";
        assert!(falls_through(&body(src), &exits_at(src, &[], &["match c:"])));
    }

    #[test]
    fn test_exit_call_in_a_with_body_may_be_suppressed() {
        // `with suppress(SystemExit): sys.exit(1)` completes normally.
        let src = "def f(x):\n    with m:\n        stop(1)\n";
        assert!(falls_through(&body(src), &exits_at(src, &["stop(1)"], &[])));
        // The exit nested in an `if`: `x` is None after the suppressed exit.
        let src = "def f(x):\n    with m:\n        if x is None:\n            stop(1)\n    use()\n";
        let exits = exits_at(src, &["stop(1)"], &[]);
        assert_eq!(narrowing_with_exits(src, &exits), vec![Vec::<String>::new()]);
        // The same for a marked match.
        let src = "def f(x, c):\n    with m:\n        if x is None:\n            match c:\n                case E.A:\n                    return 1\n    use()\n";
        let exits = exits_at(src, &[], &["match c:"]);
        assert_eq!(narrowing_with_exits(src, &exits), vec![Vec::<String>::new()]);
    }

    #[test]
    fn test_raise_in_a_with_body_may_be_suppressed() {
        // `with suppress(ValueError): raise ValueError(k)` completes normally.
        assert!(falls_through(&body("def f(k):\n    with m:\n        raise E(k)\n"), &Exits::default()));
        let src = "def f(x):\n    with m:\n        if x:\n            return 1\n        else:\n            raise E()\n";
        assert!(falls_through(&body(src), &Exits::default()));
        let src = "def f(x):\n    with m:\n        while True:\n            g()\n";
        assert!(falls_through(&body(src), &Exits::default()));
        let src = "def f(x):\n    with m:\n        with n:\n            raise E()\n";
        assert!(falls_through(&body(src), &Exits::default()));
        // A `return` leaves whatever the manager does.
        assert!(!falls_through(&body("def f(x):\n    with m:\n        return x\n"), &Exits::default()));
        let src = "def f(x):\n    with m:\n        if x:\n            return 1\n        else:\n            return 2\n";
        assert!(!falls_through(&body(src), &Exits::default()));
        // Outside a `with` the same bodies end the flow.
        assert!(!falls_through(&body("def f(k):\n    raise E(k)\n"), &Exits::default()));
        assert!(!falls_through(&body("def f(x):\n    while True:\n        g()\n"), &Exits::default()));
    }

    #[test]
    fn test_with_body_narrowing_does_not_survive_it() {
        let after_with = |inner: &str| {
            let src = format!("def f(x):\n    with m:\n        {inner}\n        use()\n    use()\n");
            narrowing_at_uses(&src)
        };
        let x = vec!["x".to_string()];
        let none = Vec::<String>::new();
        // Inside the body the narrowing holds, after it the manager may have
        // suppressed the exception that the None case raises.
        assert_eq!(after_with("assert x is not None"), vec![x.clone(), none.clone()]);
        assert_eq!(after_with("if x is None:\n            raise E()"), vec![x.clone(), none.clone()]);
        assert_eq!(after_with("x.upper()"), vec![x.clone(), none.clone()]);
        // `g()` may raise before the guard runs.
        assert_eq!(after_with("g()\n        if x is None:\n            return"), vec![x.clone(), none.clone()]);
        // A narrowing from before the `with` survives it.
        let src = "def f(x):\n    assert x is not None\n    with m:\n        g()\n    use()\n";
        assert_eq!(narrowing_at_uses(src), vec![x.clone()]);
        // But not when the body rebinds the name.
        let src = "def f(x):\n    assert x is not None\n    with m:\n        x = g()\n    use()\n";
        assert_eq!(narrowing_at_uses(src), vec![none.clone()]);
    }

    #[test]
    fn test_later_with_item_narrowing_does_not_survive_it() {
        let x = vec!["x".to_string()];
        let none = Vec::<String>::new();
        // The second item runs inside `suppress`, which may swallow its error.
        let src = "def f(x):\n    with m, x.open():\n        pass\n    use()\n";
        assert_eq!(narrowing_at_uses(src), vec![none.clone()]);
        // The first item runs outside every manager, so its narrowing holds.
        let src = "def f(x):\n    with x.open(), m:\n        pass\n    use()\n";
        assert_eq!(narrowing_at_uses(src), vec![x]);
    }

    #[test]
    fn test_with_body_effects_reach_past_it() {
        // `d.pop(k)` may have run before a suppressed exception, so `k in d`
        // no longer holds after the `with`.
        let src = "def f(d, k):\n    if k in d:\n        with m:\n            d.pop(k)\n            g()\n        probe.get()\n";
        assert_eq!(narrowing_at_uses(src), vec![Vec::<String>::new()]);
        let src = "def f(d, k):\n    if k in d:\n        with m:\n            pass\n        probe.get()\n";
        assert_eq!(narrowing_at_uses(src), vec![vec!["d[k]".to_string()]]);
    }

    #[test]
    fn test_isinstance_narrows_in_the_true_branch_only() {
        let src = "def f(x, c):\n    if isinstance(x, str):\n        use()\n    else:\n        use()\n    use()\n";
        assert_eq!(narrowing_with_isinstance(src), vec![vec!["x".to_string()], vec![], vec![]]);
        // A call that resolution did not mark narrows nothing.
        assert_eq!(narrowing_at_uses(src), vec![Vec::<String>::new(), vec![], vec![]]);
    }

    #[test]
    fn test_isinstance_negated_and_composed() {
        let src = "def f(x, y, c):\n    if not isinstance(x, str):\n        use()\n        return\n    use()\n    if isinstance(y, int) or c:\n        use()\n    if isinstance(y, int) and c:\n        use()\n    if not isinstance(y, int) or c:\n        use()\n    else:\n        use()\n";
        let names = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            narrowing_with_isinstance(src),
            vec![
                names(&[]),
                names(&["x"]),
                names(&["x"]),
                names(&["c", "x", "y"]),
                names(&["x"]),
                names(&["x", "y"]),
            ]
        );
    }

    #[test]
    fn test_isinstance_assert_while_and_elif() {
        let src = "def f(x, y, c):\n    while isinstance(y, str):\n        use()\n    use()\n    if c:\n        use()\n    elif isinstance(y, str):\n        use()\n    assert isinstance(x, str)\n    use()\n";
        let names = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            narrowing_with_isinstance(src),
            vec![names(&["y"]), names(&[]), names(&["c"]), names(&["y"]), names(&["x"])]
        );
    }

    #[test]
    fn test_isinstance_narrows_what_is_not_none_narrows() {
        // A chained subject is the same flow name as for `is not None`, so a
        // rebinding or a write of a link ends both narrowings the same way.
        for subject in ["x", "o.a", "o.a.b.c"] {
            for tail in ["", "    o.a = c\n    use()\n", "    o = c\n    use()\n", "    x = c\n    use()\n"] {
                let body = |test: &str| format!("def f(x, o, c):\n    if not ({test}):\n        return\n    use()\n{tail}");
                let by_isinstance = narrowing_with_isinstance(&body(&format!("isinstance({subject}, str)")));
                let by_none_test = narrowing_at_uses(&body(&format!("{subject} is not None")));
                assert_eq!(by_isinstance, by_none_test, "{subject} / {tail}");
                assert!(by_isinstance[0].contains(&subject.to_string()), "{subject}");
            }
        }
    }

    #[test]
    fn test_isinstance_on_an_attribute_ends_where_a_caller_guard_ends() {
        // From the test on, the conditions of a caller's guard on `p.f` or
        // `p.a.f` (`Scope::field_read_facts`): no code handed the root, and for
        // a chain neither an attribute of the root nor a proper prefix.
        let cases = [
            ("o.a.b", "clear(o.a)", false),
            ("o.a.b.c", "clear(o.a.b)", false),
            ("o.a.b", "clear(o)", false),
            ("o.a.b", "o.a.reset()", false),
            ("o.a.b", "super().reset()", false),
            ("o.a", "clear(o)", false),
            ("o.a", "o.reset()", false),
            ("o.a.b", "keep(o.a.b)", true),
            ("o.a.b", "clear(c)", true),
            ("o.a", "clear(o.a)", true),
        ];
        for (subject, call, kept) in cases {
            let src = format!("def f(o, c):\n    if isinstance({subject}, str):\n        {call}\n        use()\n");
            let at_use = narrowing_with_isinstance(&src);
            assert_eq!(at_use[0].contains(&subject.to_string()), kept, "{subject} / {call}");
        }
        // An `is not None` test on the same subject keeps its narrowing.
        let src = "def f(o, c):\n    if isinstance(o.a.b, str) and o.a.b is not None:\n        clear(o.a)\n        use()\n";
        assert!(narrowing_with_isinstance(src)[0].contains(&"o.a.b".to_string()));
        // A branch that hands the link off ends the narrowing after the join.
        let src = "def f(o, c):\n    assert isinstance(o.a.b, str)\n    if c:\n        clear(o.a)\n    use()\n";
        assert_eq!(narrowing_with_isinstance(src), vec![Vec::<String>::new()]);
    }

    #[test]
    fn test_isinstance_on_an_attribute_needs_the_guard_intact_since_the_entry() {
        // A hand-off, an alias or a `__dict__` read before the test lets code
        // write the field after it, as a caller's guard (counted from the
        // entry) would end there. Without one the test narrows.
        let cases = [
            ("", "o.a", true),
            ("    q = o\n", "o.a", false),
            ("    q = o\n", "o.a.b", false),
            ("    a = o.a\n", "o.a.b", false),
            ("    a = o.a\n", "o.a", true),
            ("    reset = lambda: clear(o.a)\n", "o.a.b", false),
            ("    reset = lambda: clear(o)\n", "o.a", false),
            ("    t = o.__dict__\n", "o.a", false),
            ("    t = o.b.__dict__\n", "o.a.c", false),
            ("    o.a = c\n", "o.a", true),
            ("    o = c\n", "o.a", false),
            ("    keep(c)\n", "o.a.b", true),
            ("    if c:\n        q = o\n", "o.a", false),
        ];
        for (before, subject, narrowed) in cases {
            let src = format!("def f(o, c):\n{before}    if isinstance({subject}, str):\n        use()\n");
            let at_use = narrowing_with_isinstance(&src);
            assert_eq!(at_use[0].contains(&subject.to_string()), narrowed, "{before:?} / {subject}");
        }
        // Not a parameter of the function: nothing says its fields are untouched.
        let src = "def f(c):\n    o = c\n    if isinstance(o.a, str):\n        use()\n";
        assert!(!narrowing_with_isinstance(src)[0].contains(&"o.a".to_string()));
        // The elif and the assert follow the same rule.
        let src = "def f(o, c):\n    q = o\n    if c:\n        pass\n    elif isinstance(o.a, str):\n        use()\n";
        assert!(!narrowing_with_isinstance(src)[0].contains(&"o.a".to_string()));
        let src = "def f(o, c):\n    q = o\n    assert isinstance(o.a, str)\n    use()\n";
        assert!(!narrowing_with_isinstance(src)[0].contains(&"o.a".to_string()));
    }

    #[test]
    fn test_a_dict_read_after_the_test_ends_an_isinstance_narrowing() {
        for (subject, tail) in [
            ("o.a", "o.__dict__['a'] = None"),
            ("o.a.b", "o.a.__dict__['b'] = None"),
            ("o.a.b", "o.__dict__.update(c)"),
        ] {
            let src = format!("def f(o, c):\n    if isinstance({subject}, str):\n        {tail}\n        use()\n");
            assert!(!narrowing_with_isinstance(&src)[0].contains(&subject.to_string()), "{subject} / {tail}");
        }
    }

    #[test]
    fn test_a_walrus_in_the_test_ends_an_isinstance_narrowing_of_what_it_rebinds() {
        let names = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        for (test, subject) in [
            ("isinstance(x, str) and (x := c) is None", "x"),
            ("isinstance(o.a, str) and (o := c) is None", "o.a"),
            ("isinstance(o.a.b, str) and (o := c) is None", "o.a.b"),
        ] {
            let src = format!("def f(x, o, c):\n    if {test}:\n        use()\n");
            assert!(!narrowing_with_isinstance(&src)[0].contains(&subject.to_string()), "{test}");
        }
        // The same tests without the walrus narrow.
        for (test, subject) in [("isinstance(x, str)", "x"), ("isinstance(o.a, str)", "o.a")] {
            let src = format!("def f(x, o, c):\n    if {test}:\n        use()\n");
            assert!(narrowing_with_isinstance(&src)[0].contains(&subject.to_string()), "{test}");
        }
        // Rebinding another name leaves it, and so does a walrus before the
        // subject is tested.
        let src = "def f(x, c):\n    if isinstance(x, str) and (y := c) is None:\n        use()\n";
        assert_eq!(narrowing_with_isinstance(src), vec![names(&["x"])]);
        // An `is not None` test is unchanged by this rule.
        let src = "def f(x, c):\n    if x is not None and (x := c) is None:\n        use()\n";
        assert_eq!(narrowing_at_uses(src), vec![names(&["x"])]);
    }

    #[test]
    fn test_a_store_the_walk_does_not_track_ends_an_isinstance_narrowing() {
        // A nested body that writes a field of the root, rebinds the root
        // (`nonlocal`) or reads its `__dict__` can run any time after its
        // definition, whether the definition is before the test or in the branch.
        let cases = [
            ("    def g():\n        o.a = None\n", "o.a", false),
            ("    def g():\n        o.a.b = None\n", "o.a.b", false),
            ("    def g():\n        o.__dict__['a'] = None\n", "o.a", false),
            ("    def g():\n        nonlocal o\n        o = c\n", "o.a", false),
            ("    class K:\n        o.a = None\n", "o.a", false),
            ("    def g():\n        return o.a\n", "o.a", true),
            ("    def g(o):\n        o.a = None\n", "o.a", true),
        ];
        for (before, subject, narrowed) in cases {
            let src = format!("def f(o, c):\n{before}    if isinstance({subject}, str):\n        use()\n");
            assert_eq!(narrowing_with_isinstance(&src)[0].contains(&subject.to_string()), narrowed, "{before:?}");
        }
        for def in ["def g():\n            o.a = None\n", "class K:\n            o.a = None\n"] {
            let src = format!("def f(o, c):\n    if isinstance(o.a, str):\n        {def}        use()\n");
            assert!(!narrowing_with_isinstance(&src)[0].contains(&"o.a".to_string()), "{def:?}");
        }
        // A comprehension or `for` target that is an attribute of the root.
        for test in ["any(0 for o.a in c)", "any(0 for o.a, d in c)", "any(0 for d, o.a in c)", "any(0 for d in c for o.a in d)"] {
            let src = format!("def f(o, c):\n    if isinstance(o.a, str) and {test}:\n        use()\n");
            assert!(!narrowing_with_isinstance(&src)[0].contains(&"o.a".to_string()), "{test}");
        }
    }

    #[test]
    fn test_a_walrus_after_the_test_ends_an_isinstance_narrowing_of_what_it_rebinds() {
        // The walrus is in a later node, not in the test (`Ctx::after_operand`
        // reaches the same `Narrowed::apply`).
        for subject in ["x", "o.a"] {
            let root = subject.split('.').next().unwrap();
            let src = format!("def f(x, o, c):\n    if isinstance({subject}, str):\n        y = ({root} := c)\n        use()\n");
            assert!(!narrowing_with_isinstance(&src)[0].contains(&subject.to_string()), "{subject}");
            // Another name, or no walrus, keeps it.
            let src = format!("def f(x, o, c):\n    if isinstance({subject}, str):\n        y = (z := c)\n        use()\n");
            assert!(narrowing_with_isinstance(&src)[0].contains(&subject.to_string()), "{subject}");
        }
        // A name that an `is not None` test also narrows keeps that narrowing.
        let src = "def f(x, c):\n    if x is not None and isinstance(x, str):\n        y = (z := c)\n        use()\n";
        assert!(narrowing_with_isinstance(src)[0].contains(&"x".to_string()));
    }

    #[test]
    fn test_a_walrus_or_a_hand_off_in_the_same_statement_ends_an_isinstance_narrowing() {
        // `use(...)` observes the state its own operands are read against.
        let cases = [
            ("o.a", "use(clear(o), o.a)", false),
            ("o.a", "use(o.a, clear(o))", false),
            ("o.a", "use(o.__dict__, o.a)", false),
            ("o.a", "use(o.a)", true),
            ("o.a", "use(clear(c), o.a)", true),
            ("x", "use((x := c), x)", false),
            ("x", "use(x, (x := c))", false),
            ("x", "use((z := c), x)", true),
            ("o.a", "use((o := c), o.a)", false),
        ];
        for (subject, stmt, narrowed) in cases {
            let src = format!("def f(x, o, c):\n    if isinstance({subject}, str):\n        {stmt}\n");
            assert_eq!(narrowing_with_isinstance(&src)[0].contains(&subject.to_string()), narrowed, "{subject} / {stmt}");
        }
        // A suspension anywhere in the statement ends it too.
        let src = "async def f(o, c):\n    if isinstance(o.a, str):\n        use(await c, o.a)\n";
        assert!(!narrowing_with_isinstance(src)[0].contains(&"o.a".to_string()));
        // The same in an assert, and in the test of an if and of an elif, which
        // are read before the branch (`use` is observed by the header here).
        let src = "def f(o, c):\n    assert isinstance(o.a, str)\n    use(clear(o), o.a)\n";
        assert!(!narrowing_with_isinstance(src)[0].contains(&"o.a".to_string()));
        let src = "def f(o, c):\n    assert isinstance(o.a, str)\n    if use(clear(o), o.a):\n        pass\n    elif use(clear(o), o.a):\n        pass\n";
        assert!(narrowing_with_isinstance(src).iter().all(|at| !at.contains(&"o.a".to_string())));
    }

    #[test]
    fn test_a_hand_off_of_a_link_in_an_earlier_argument_is_not_yet_reached() {
        // `o.a` handed in an earlier argument cannot be told from a read of the
        // chain `o.a.b` in the same call, so the narrowing stays (README).
        let src = "def f(o, c):\n    if isinstance(o.a.b, str):\n        use(clear(o.a), o.a.b)\n";
        assert!(narrowing_with_isinstance(src)[0].contains(&"o.a.b".to_string()));
    }

    #[test]
    fn test_a_tracked_store_before_the_test_does_not_stop_an_isinstance_narrowing() {
        // The walk follows these stores through `bound_names`, so they only
        // forget what they write.
        for before in ["    o.b: int = 0\n", "    o.b, o.d = c, c\n", "    for o.b in c:\n        pass\n", "    with c as o.b:\n        pass\n", "    o.b += c\n"] {
            let src = format!("def f(o, c):\n{before}    if isinstance(o.a, str):\n        use()\n");
            assert!(narrowing_with_isinstance(&src)[0].contains(&"o.a".to_string()), "{before:?}");
        }
        // After the test, a store to the subject itself ends it.
        for after in ["o.a: str = c", "o.a, o.d = c, c", "o.a += c", "del o.a"] {
            let src = format!("def f(o, c):\n    if isinstance(o.a, str):\n        {after}\n        use()\n");
            assert!(!narrowing_with_isinstance(&src)[0].contains(&"o.a".to_string()), "{after}");
        }
        for after in ["for o.a in c:\n            pass", "with c as o.a:\n            pass"] {
            let src = format!("def f(o, c):\n    if isinstance(o.a, str):\n        {after}\n        use()\n");
            assert!(!narrowing_with_isinstance(&src)[0].contains(&"o.a".to_string()), "{after}");
        }
    }

    #[test]
    fn test_isinstance_narrows_only_a_name_or_a_chain() {
        // A call on the result starts at the marked offset but is not the test.
        let src = "def f(x, y):\n    if isinstance(x, str).__eq__(y):\n        use()\n";
        assert_eq!(narrowing_with_isinstance(src), vec![Vec::<String>::new()]);
        // `d[0]` dereferences `d`, which narrows it whatever the test.
        let src = "def f(x, d):\n    if isinstance(d[0], str):\n        use()\n    if isinstance(g(x), str):\n        use()\n";
        assert_eq!(narrowing_with_isinstance(src), vec![vec!["d".to_string()], vec!["d".to_string()]]);
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

    /// Round 6: a dereference that certainly runs narrows for the rest of the block.
    #[test]
    fn test_dereference_narrowing() {
        let uses = narrowing_at_uses(
            "def f(a, b, c, d, e, g, h, k):\n    a.m()\n    use()\n    b[0]\n    len(c)\n    use()\n    d and d.x\n    e.__class__\n    use()\n    if g.flag:\n        pass\n    use()\n    h.x = 1\n    k = k.strip()\n    use()\n",
        );
        assert_eq!(
            uses,
            vec![
                vec!["a".to_string()],
                vec!["a".into(), "b".into(), "c".into()],
                // `d and d.x` does not dereference d; `__class__` exists on None.
                vec!["a".into(), "b".into(), "c".into()],
                vec!["a".into(), "b".into(), "c".into(), "g".into()],
                // `k` is rebound by the statement that dereferences it.
                vec!["a".into(), "b".into(), "c".into(), "g".into(), "h".into()],
            ]
        );
        // On one branch only: not after the `if`.
        let uses = narrowing_at_uses("def f(x, c):\n    if c:\n        x.m()\n    use()\n");
        assert_eq!(uses, vec![Vec::<String>::new()]);
        // `obj.f.g` narrows the attribute `obj.f` and `obj`.
        let uses = narrowing_at_uses("def f(obj):\n    obj.f.g()\n    use()\n");
        assert_eq!(uses, vec![vec!["obj".to_string(), "obj.f".into()]]);
    }

    #[test]
    fn test_no_narrowing_without_exit_or_across_loops() {
        let uses = narrowing_at_uses(
            "def f(x, xs):\n    if x is None:\n        log()\n    use()\n    assert x\n    for i in xs:\n        use()\n        x = h(i)\n",
        );
        assert_eq!(uses, vec![Vec::<String>::new(), vec![]]);
    }

    /// After a `try`: the join of the body (with `else`) and each handler
    /// that falls through; a `finally` starts from that joined with the
    /// state before the `try`.
    #[test]
    fn test_try_joins_fall_through_paths() {
        let at = |handler: &str| {
            narrowing_at_uses(&format!(
                "def f(x):\n    try:\n        assert x\n    except E:\n        {handler}\n    use()\n"
            ))
        };
        assert_eq!(at("return"), vec![vec!["x".to_string()]]);
        assert_eq!(at("assert x"), vec![vec!["x".to_string()]]);
        assert_eq!(at("pass"), vec![Vec::<String>::new()]);
        let uses = narrowing_at_uses(
            "def f(x, y):\n    try:\n        pass\n    except E:\n        return\n    else:\n        assert x\n    use()\n",
        );
        assert_eq!(uses, vec![vec!["x".to_string()]]);
        let uses = narrowing_at_uses(
            "def f(x, y):\n    try:\n        assert x\n    except E:\n        return\n    finally:\n        use()\n        assert y\n    use()\n",
        );
        assert_eq!(uses, vec![vec![], vec!["y".to_string()]]);
        // A body that always exits, a handler that falls through.
        let uses = narrowing_at_uses(
            "def f(x):\n    try:\n        return 1\n    except E:\n        assert x\n    use()\n",
        );
        assert_eq!(uses, vec![vec!["x".to_string()]]);
        // A body that falls through, a handler that exits, and a nested try.
        let uses = narrowing_at_uses(
            "def f(x):\n    try:\n        try:\n            assert x\n        except E:\n            return\n    except F:\n        return\n    use()\n",
        );
        assert_eq!(uses, vec![vec!["x".to_string()]]);
    }

    #[test]
    fn test_literal_membership_narrows() {
        let uses = narrowing_at_uses(
            "def f(a, b, c, d, o):\n    if a in {'x', 'y'}:\n        use()\n    if b not in ('x', 1):\n        return\n    use()\n    if c in {'x', None}:\n        use()\n    if o.f in [True, b'z']:\n        use()\n    if d in []:\n        use()\n",
        );
        assert_eq!(
            uses,
            vec![
                vec!["a".to_string()],
                vec!["b".into()],
                vec!["b".into()],
                // The test dereferences `o`.
                vec!["b".into(), "o".into(), "o.f".into()],
                vec!["b".into(), "o".into()],
            ]
        );
    }

    #[test]
    fn test_member_facts() {
        let uses = narrowing_at_uses(
            "def f(d, k, e, j, o):\n    if k in d:\n        v = d.get(k)\n        use()\n        use()\n    if k not in d:\n        return\n    use()\n    if 'a' in o.m:\n        use()\n    if k in d and g():\n        use()\n",
        );
        assert_eq!(
            uses,
            vec![
                // `d.get` dereferences `d`; the call `use()` drops `d[k]`.
                vec!["d".to_string(), "d[k]".into()],
                vec!["d".into()],
                vec!["d[k]".into()],
                vec!["o".into(), "o.m[\"a\"]".into()],
                vec!["o".into()],
            ]
        );
        for rebind in ["d = e", "k = j", "for d in e:\n            pass"] {
            let uses = narrowing_at_uses(&format!(
                "def f(d, k, e, j):\n    if k in d:\n        {rebind}\n        use()\n"
            ));
            assert_eq!(uses, vec![Vec::<String>::new()], "{rebind}");
        }
        let uses = narrowing_at_uses(
            "def f(o, k, p):\n    if k in o.m:\n        o = p\n        use()\n",
        );
        assert_eq!(uses, vec![Vec::<String>::new()]);
        let uses = narrowing_at_uses("def f(d, k):\n    if k in d:\n        if g():\n            use()\n");
        assert_eq!(uses, vec![Vec::<String>::new()]);
        // Statements that can remove or rebind the key without a plain call
        // in the narrowed branch's own test.
        let no_members = |code: &str| {
            let uses = narrowing_at_uses(code);
            assert!(!uses.is_empty(), "{code}: no use() reached");
            assert!(
                uses.iter().all(|u| u.iter().all(|n| !n.contains('['))),
                "{code}: {uses:?}"
            );
        };
        for body in [
            "del d[k]\n        use()",
            "if (k := j):\n            pass\n        use()",
            "match mode:\n            case 1:\n                d.pop(k)\n        use()",
            "match mode:\n            case 1 if g():\n                pass\n        use()",
            "try:\n            d.pop(k)\n            h()\n        except E:\n            probe.get()",
            "try:\n            x = 1\n        except E:\n            d.pop(k)\n            raise\n        finally:\n            probe.get()",
            "try:\n            x = 1\n        except E:\n            return 0\n        else:\n            d.pop(k)\n            return 1\n        finally:\n            probe.get()",
            "class C:\n            x = d.pop(k)\n        use()",
            "@d.pop(k)\n        def g():\n            pass\n        use()",
            "def g(x=d.pop(k)):\n            pass\n        use()",
            "def g() -> d.pop(k):\n            pass\n        use()",
            "await fut\n        use()",
            "yield 1\n        use()",
            "yield from it\n        use()",
            "xs = [x async for x in it]\n        use()",
            "xs = {x: x async for x in it}\n        use()",
            "xs = {x async for x in it}\n        use()",
            "xs = [w for x in it async for y in x for w in y]\n        use()",
            "xs = (y for x in it for y in x if True for z in x async for w in z)\n        use()",
            "if [x async for x in it]:\n            pass\n        use()",
            "async with cm:\n            use()",
            "async for x in it:\n            probe.get()",
            "try:\n            async with cm:\n                pass\n        except E:\n            probe.get()",
            "try:\n            async for x in it:\n                pass\n        except E:\n            probe.get()",
            "assert g()\n        use()",
            "with g():\n            use()",
            "match g():\n            case 1:\n                pass\n        use()",
            "if a:\n            pass\n        elif g():\n            use()",
        ] {
            no_members(&format!(
                "def f(d, k, j, a, mode):\n    if k in d:\n        {body}\n"
            ));
        }
        // A member fact from an earlier test of an `elif` chain, then a call.
        no_members("def f(d, k):\n    if k not in d:\n        return\n    elif g():\n        return\n    use()\n");
        no_members("def f(d, k, a):\n    if k not in d:\n        return\n    if a:\n        pass\n    elif g():\n        return\n    use()\n");
        // A chained comparison says nothing about its first pair.
        for code in [
            "def f(d, e, k):\n    if k not in d not in e:\n        return\n    use()\n",
            "def f(x, y):\n    if x is None is y:\n        return\n    use()\n",
            "def f(x, y):\n    if x in {'a'} == y:\n        use()\n",
        ] {
            assert_eq!(narrowing_at_uses(code), vec![Vec::<String>::new()], "{code}");
        }
        // The negative side of an `or` test with a call.
        no_members("def f(d, k):\n    if k not in d or g():\n        return\n    use()\n");
        no_members("def f(d, k, a):\n    if a:\n        return\n    elif k not in d or g():\n        return\n    use()\n");
        // Only the dict's own `.get` is known to leave its keys alone.
        let after_get = |receiver: &str| {
            narrowing_at_uses(&format!(
                "def f(d, e, k, j):\n    if k in d:\n        {receiver}.get(j)\n        probe.get()\n"
            ))
        };
        assert_eq!(after_get("d"), vec![vec!["d".to_string(), "d[k]".into()]]);
        assert_eq!(after_get("e"), vec![vec!["e".to_string()]]);
        // A call in one iteration may remove the key before the next.
        for lp in ["for x in e:", "while x:"] {
            let uses = narrowing_at_uses(&format!(
                "def f(d, k, e, x):\n    if k in d:\n        {lp}\n            use()\n"
            ));
            assert!(uses[0].iter().all(|n| !n.contains('[')), "{lp}: {uses:?}");
        }
    }

    #[test]
    fn test_while_test_reads_the_loops_effects() {
        // The test of a `while` runs again after every iteration, so the
        // header sees a key fact only when nothing in the loop may pop it.
        fn at_while_test(src: &str) -> Vec<String> {
            struct V(Vec<Vec<String>>);
            impl<'a> FlowVisitor<'a> for V {
                fn simple(&mut self, _: &'a Stmt, _: &Narrowed) {}
                fn header(&mut self, _: &'a Expr, n: &Narrowed) {
                    let mut names: Vec<String> = n.names().cloned().collect();
                    names.sort();
                    self.0.push(names);
                }
            }
            let b = body(src);
            let mut v = V(Vec::new());
            walk_block(&b, &Narrowed::new(), &Exits::default(), &mut v);
            // The first header is the `if` test; the last is the `while` test.
            v.0.pop().expect("a while test")
        }
        let loop_with = |inner: &str| {
            at_while_test(&format!(
                "def f(d, k):\n    if k in d:\n        while d.get(k):\n            {inner}\n"
            ))
        };
        assert_eq!(loop_with("pass"), vec!["d[k]".to_string()]);
        assert_eq!(loop_with("d.pop(k)"), Vec::<String>::new());
    }

    #[test]
    fn test_falls_through() {
        assert!(!falls_through(&body("def f(x):\n    return x\n"), &Exits::default()));
        assert!(falls_through(&body(
            "def f(x):\n    if x:\n        return 1\n"
        ), &Exits::default()));
        assert!(!falls_through(&body(
            "def f(x):\n    if x:\n        return 1\n    else:\n        raise E()\n"
        ), &Exits::default()));
        assert!(!falls_through(&body(
            "def f(x):\n    while True:\n        return 1\n"
        ), &Exits::default()));
        assert!(falls_through(&body(
            "def f(x):\n    while True:\n        break\n"
        ), &Exits::default()));
        assert!(!falls_through(&body(
            "def f(x):\n    try:\n        return 1\n    except E:\n        return 2\n"
        ), &Exits::default()));
        // Round 5 N1: an exhaustive `match` whose every case returns or raises.
        assert!(!falls_through(&body(
            "def f(x):\n    match x:\n        case 'a':\n            return 1\n        case _:\n            raise E()\n"
        ), &Exits::default()));
        assert!(!falls_through(&body(
            "def f(x):\n    match x:\n        case 1 | other:\n            return 1\n"
        ), &Exits::default()));
        assert!(falls_through(&body(
            "def f(x):\n    match x:\n        case 'a':\n            return 1\n"
        ), &Exits::default()));
        assert!(falls_through(&body(
            "def f(x):\n    match x:\n        case _ if x:\n            return 1\n"
        ), &Exits::default()));
        assert!(falls_through(&body(
            "def f(x):\n    match x:\n        case 'a':\n            pass\n        case _:\n            return 2\n"
        ), &Exits::default()));
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
