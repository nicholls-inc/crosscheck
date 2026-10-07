//! Edge discovery (data-flow model v2, `docs/design/dataflow-v2.md`).
//!
//! | Code | Edge |
//! |------|------|
//! | `Cls(f=g(...))`, `x = g(...); Cls(f=x)`, likewise `objects.create`, `**{...}`, positional, `obj.f = g(...)` | `g@site writes_to Cls.f` |
//! | `Cls(f=expr)` etc., `expr` not a call to an extracted function | `F writes_to Cls.f`, override = contracts of `expr` in F |
//! | `h(..., g(...), ...)`, or via a local bound to `g(...)` | `g@site flows_to h` and `g@site flows_to h@site`, `target_param` = the parameter it binds |
//! | `h(..., expr, ...)`, `expr` not such a call | `F flows_to h` (and `h@site`), `target_param` set, override = contracts of `expr` |
//! | `h(...)` | `F calls h` (structural, not checked) |
//!
//! `F` is the enclosing function (module-level code is the pseudo function
//! `<module m>`). `g@site` is the call-site node of one call of `g` whose
//! value is consumed (written, passed as an argument, or returned): the
//! result of `g` at that call. Consumer edges leave the call-site node, never
//! `g`'s own node, so two unrelated calls of one helper are not joined.
//! Argument edges go to both the callee's own node (for the writes it makes
//! from its parameters) and its call-site node (for its result there).
//!
//! Names are resolved through `resolve`; a call or class that does not
//! resolve to something extracted gives no edge.

use std::collections::HashMap;

use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, BoolOp, Expr, Stmt};
use ruff_text_size::Ranged;
use serde::Deserialize;

use crate::db::{ConstraintType, ContractRecord, VerificationLevel};
use crate::extractor::Project;
use crate::flow::{self, FlowVisitor, FunctionFlow, Narrowed};
use crate::function_extractor::{self, FunctionInfo, ParamKind};
use crate::resolve::{ClassInfo, ProjectIndex};
use crate::source::LineIndex;
use crate::value_analysis::{facts_rows, Alt, Callee, Ctx, Scope, SplatValue, ValueFacts};

/// A call expression: the key of its call-site node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SiteKey {
    pub file: String,
    /// Byte offset of the call expression.
    pub offset: u32,
    /// Qualified name of the callee (a call that dispatches to several
    /// methods has one call-site node per target).
    pub callee: String,
}

/// A call-site node: the result of `callee` at one call.
#[derive(Debug, Clone)]
pub struct CallSite {
    pub key: SiteKey,
    /// Qualified name of the called function.
    pub callee: String,
    /// 1-based line of the call expression.
    pub line: u32,
}

/// A discovered edge between two nodes in the contract graph.
#[derive(Debug, Clone)]
pub struct DiscoveredEdge {
    /// Qualified name of the source function (for a call-site source, the callee).
    pub source_function: String,
    /// Qualified name of the target function, or of the target class when
    /// `target_field` is set.
    pub target_name: String,
    pub target_field: Option<String>,
    pub relationship: String,
    pub discovery: String,
    /// For `flows_to`: the callee parameter the value binds.
    pub target_param: Option<String>,
    /// `Some(rows)`: on this edge the source's postconditions are `rows`
    /// (`source_override = 1`); their `node_id` / `edge_id` are set on insert.
    pub override_rows: Option<Vec<ContractRecord>>,
    /// The source is the call-site node of this call.
    pub source_site: Option<SiteKey>,
    /// A `flows_to` into the call-site node of this call (else the callee's own node).
    pub target_site: Option<SiteKey>,
    /// `(file, line)` of the write or call expression that produced the edge.
    pub site: Option<(String, u32)>,
}

impl DiscoveredEdge {
    fn new(source: &str, target: &str, field: Option<&str>, relationship: &str) -> Self {
        DiscoveredEdge {
            source_function: source.to_string(),
            target_name: target.to_string(),
            target_field: field.map(str::to_string),
            relationship: relationship.to_string(),
            discovery: "ast_pattern".to_string(),
            target_param: None,
            override_rows: None,
            source_site: None,
            target_site: None,
            site: None,
        }
    }
}

/// Every edge of a project and the call-site nodes they use.
#[derive(Debug, Default)]
pub struct Discovered {
    pub edges: Vec<DiscoveredEdge>,
    /// Call sites whose result is consumed, in discovery order.
    pub call_sites: Vec<CallSite>,
}

/// Discover edges in a single module, as the unnamed module `""` (so
/// qualified names are the bare names). Convenience for tests.
pub fn discover_edges(stmts: &[Stmt]) -> Vec<DiscoveredEdge> {
    let project = Project::from_parsed(vec![crate::extractor::SourceModule::from_stmts(
        "",
        "",
        stmts.to_vec(),
    )]);
    discover_project_edges(&project)
}

/// Discover the edges of every extracted function in the project.
pub fn discover_project_edges(project: &Project) -> Vec<DiscoveredEdge> {
    discover(project).edges
}

/// Discover every edge and call-site node of the project.
pub fn discover(project: &Project) -> Discovered {
    let forwards = kw_forwards(&project.index, &project.summaries);
    let dict_forwarders: std::collections::HashSet<String> = forwards
        .keys()
        .filter(|k| k.ends_with(')'))
        .cloned()
        .collect();
    // Measurement knob: without call-site nodes, results leave the callee's
    // own node (the round-2 model), to compare graph sizes and path counts.
    let call_sites = std::env::var_os("CROSSCHECK_NO_CALL_SITES").is_none();
    let mut out = Discovered::default();
    let mut uncalled_forwarders: Vec<(String, DiscoveredEdge)> = Vec::new();
    let empty = LineIndex::default();
    for func in &project.index.functions {
        let flow = FunctionFlow::of_info(func);
        let scope = Scope::new(&project.index, &project.summaries, Some(func), &flow)
            .with_dict_forwarders(&dict_forwarders);
        let doc = project
            .docstrings
            .get(&func.qualified_name)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let mut walker = EdgeWalker {
            project,
            scope: &scope,
            func,
            lines: project.lines_of(&func.source_file).unwrap_or(&empty),
            doc_rows: function_extractor::docstring_postcondition_rows(func, doc, 0),
            forwards: &forwards,
            return_call: None,
            edges: Vec::new(),
            used: HashMap::new(),
            call_sites,
            attr_counts: HashMap::new(),
            pending: Vec::new(),
            observations: Vec::new(),
            forwarder_edges: Vec::new(),
        };
        let end = flow::walk_block(&func.body, &flow.entry, &flow.exits, &mut walker);
        if flow.falls_through {
            walker.observations.push((None, end));
            // Falling off the end returns None, against a non-Optional
            // annotation (a stub body of only a docstring, `...` and `pass`
            // is a declaration, not a return site).
            let stub = func.body.iter().all(|s| match s {
                Stmt::Pass(_) => true,
                Stmt::Expr(e) => matches!(e.value.as_ref(), Expr::StringLiteral(_) | Expr::EllipsisLiteral(_)),
                _ => false,
            });
            if func.has_return_contract() && !stub {
                if let Some(last) = func.body.last() {
                    walker.none_return(last.range().end().to_u32().saturating_sub(1));
                }
            }
        }
        walker.flush_attribute_writes();
        let EdgeWalker { edges, used, forwarder_edges, .. } = walker;
        uncalled_forwarders.extend(forwarder_edges.into_iter().map(|e| (func.qualified_name.clone(), e)));
        // Argument edges into call-site nodes whose result nothing consumes are dropped.
        out.edges.extend(
            edges
                .into_iter()
                .filter(|e| e.target_site.as_ref().is_none_or(|k| used.contains_key(k))),
        );
        let mut sites: Vec<CallSite> = used.into_values().collect();
        sites.sort_by(|a, b| a.key.cmp(&b.key));
        out.call_sites.extend(sites);
    }
    // A forwarder's own `Cls(**kw)` writes whatever its callers pass; with
    // no call in the project (a framework calls it), its keys are unknown.
    let called: std::collections::HashSet<&str> = out
        .edges
        .iter()
        .filter(|e| e.relationship == "calls")
        .map(|e| e.target_name.as_str())
        .collect();
    let extra: Vec<DiscoveredEdge> = uncalled_forwarders
        .into_iter()
        .filter(|(f, _)| !called.contains(f.as_str()))
        .map(|(_, e)| e)
        .collect();
    out.edges.extend(extra);
    out
}

/// Where a function's `**kwargs` go.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Forward {
    /// Into a constructor or model write of this class (`Cls(**kw)`,
    /// `Model.objects.create(**kw)`): every keyword is a field write.
    Class(String),
    /// Into another extracted function (`g(**kw)`).
    Function(String),
}

/// Model manager methods whose keyword arguments are field values (and
/// their async forms; `abulk_create`, like `bulk_create`, takes instances
/// whose constructor calls are the writes).
const WRITE_METHODS: [&str; 8] = [
    "create",
    "update",
    "get_or_create",
    "update_or_create",
    "acreate",
    "aupdate",
    "aget_or_create",
    "aupdate_or_create",
];

/// The key of a dict parameter's forwards in `kw_forwards`: `func(param)`.
pub fn dict_param_key(func: &str, param: &str) -> String {
    format!("{func}({param})")
}

/// Where keyword arguments and dicts passed to a function go. For a
/// function with a `**kw` parameter that it only reads and passes on as
/// `**kw` (key: the function), and for a named parameter it only passes on
/// as `**param` (a dict forwarder, `def create(self, data): return
/// Model.objects.create(**data)`; key: `dict_param_key`): the constructor,
/// model write or function the entries go to.
pub fn kw_forwards(
    index: &ProjectIndex,
    summaries: &HashMap<String, ValueFacts>,
) -> HashMap<String, Vec<Forward>> {
    let mut out = HashMap::new();
    for func in &index.functions {
        let kw = func
            .params
            .iter()
            .find(|p| p.kind == ParamKind::VarKeywords)
            .map(|p| (p.name.as_str(), func.qualified_name.clone()));
        let dicts = func
            .named_value_params()
            .map(|p| (p.name.as_str(), dict_param_key(&func.qualified_name, &p.name)));
        let candidates: Vec<(&str, String)> = kw.into_iter().chain(dicts).collect();
        if candidates.is_empty() {
            continue;
        }
        let flow = FunctionFlow::of_info(func);
        let scope = Scope::new(index, summaries, Some(func), &flow);
        for (name, key) in candidates {
            if flow.escaping.contains(name) || flow.binds(name) {
                continue;
            }
            let targets = splat_targets(func, name, &scope);
            if !targets.is_empty() {
                out.insert(key, targets);
            }
        }
    }
    out
}

/// Where `func` passes `**name`: constructors of data classes and models,
/// model write methods, other extracted functions.
fn splat_targets(func: &FunctionInfo, kw: &str, scope: &Scope) -> Vec<Forward> {
    struct Splats<'k, 'a>(&'k str, Vec<&'a ast::ExprCall>);
    impl<'a> Visitor<'a> for Splats<'_, 'a> {
        fn visit_expr(&mut self, expr: &'a Expr) {
            if let Expr::Call(c) = expr {
                if c.arguments.keywords.iter().any(|k| {
                    k.arg.is_none() && matches!(&k.value, Expr::Name(n) if n.id.as_str() == self.0)
                }) {
                    self.1.push(c);
                }
            }
            visitor::walk_expr(self, expr);
        }
        fn visit_stmt(&mut self, stmt: &'a Stmt) {
            // Nested definitions have their own scope.
            if !matches!(stmt, Stmt::FunctionDef(_) | Stmt::ClassDef(_)) {
                visitor::walk_stmt(self, stmt);
            }
        }
    }
    let mut splats = Splats(kw, Vec::new());
    for s in &func.body {
        splats.visit_stmt(s);
    }
    let ctx = Ctx::default();
    let mut targets = Vec::new();
    for call in splats.1 {
        let found: Vec<Forward> = if let Some((class, method)) = scope.objects_call(call, &ctx) {
            WRITE_METHODS
                .contains(&method)
                .then(|| Forward::Class(class.qualified.clone()))
                .into_iter()
                .collect()
        } else {
            scope
                .callees(&call.func, &ctx)
                .into_iter()
                .filter_map(|c| match c {
                    Callee::Class(c) if c.kind.has_fields() => Some(Forward::Class(c.qualified.clone())),
                    Callee::Function { qualified, .. } if qualified != func.qualified_name => {
                        Some(Forward::Function(qualified))
                    }
                    _ => None,
                })
                .collect()
        };
        for t in found {
            if !targets.contains(&t) {
                targets.push(t);
            }
        }
    }
    targets
}

#[derive(Clone)]
enum Target<'a> {
    Field(&'a ClassInfo, &'a str),
    /// A callee (qualified name), the parameter the value binds, and the
    /// call-site node (`None`: the callee's own node).
    Function(&'a str, &'a str, Option<SiteKey>),
    /// The enclosing function's return contract node (`f.<return>`).
    Return,
}

struct EdgeWalker<'a, 's> {
    project: &'a Project,
    scope: &'s Scope<'a>,
    func: &'a FunctionInfo,
    lines: &'a LineIndex,
    /// The enclosing function's docstring `ensures:` rows (ASSUMED).
    doc_rows: Vec<ContractRecord>,
    forwards: &'s HashMap<String, Vec<Forward>>,
    /// The call that is the value of the `return` statement being scanned.
    return_call: Option<*const ast::ExprCall>,
    edges: Vec<DiscoveredEdge>,
    /// Call sites whose result is consumed.
    used: HashMap<SiteKey, CallSite>,
    /// Whether consumed results get call-site nodes (see `discover`).
    call_sites: bool,
    /// Simple attribute assignments seen per `obj.f` (the next `Def::Assign`
    /// index, counted as `flow::walk` counts them).
    attr_counts: HashMap<String, usize>,
    /// Field writes `obj.f = v` awaiting the points that observe them.
    pending: Vec<PendingWrite<'a>>,
    /// Where written attribute values are observed: `obj.save()` (receiver
    /// `obj`) and the function's exits (`None`), with the state there.
    observations: Vec<(Option<String>, Narrowed)>,
    /// Dynamic writes of a forwarder's own `Cls(**kw)`: kept only when the
    /// forwarder has no call in the project (see `discover`).
    forwarder_edges: Vec<DiscoveredEdge>,
}

/// A field write `obj.f = value` (a simple attribute assignment).
struct PendingWrite<'a> {
    /// `obj.f` (the flow name) and the assignment's `Def::Assign` index.
    name: String,
    index: usize,
    receiver: String,
    class: &'a ClassInfo,
    field: &'a str,
    value: &'a Expr,
    ctx: Ctx,
    site: u32,
}

impl<'a, 's> FlowVisitor<'a> for EdgeWalker<'a, 's> {
    fn simple(&mut self, stmt: &'a Stmt, narrowed: &Narrowed) {
        let ctx = Ctx::new(narrowed);
        match stmt {
            Stmt::Assign(a) => {
                self.scan(&a.value, &ctx);
                if let Some(pairs) = flow::simple_attr_targets(a) {
                    // Flow-sensitive: written when observed (see `flush_attribute_writes`).
                    for (target, (name, _)) in a.targets.iter().zip(pairs) {
                        self.scan_target(target, &ctx);
                        let count = self.attr_counts.entry(name.clone()).or_default();
                        let index = *count;
                        *count += 1;
                        self.defer_write(target, name, index, &a.value, &ctx);
                    }
                    return;
                }
                for target in &a.targets {
                    self.scan_target(target, &ctx);
                    self.assign(target, Some(&a.value), &ctx);
                }
            }
            Stmt::AnnAssign(a) => {
                if let Some(value) = &a.value {
                    self.scan(value, &ctx);
                    self.scan_target(&a.target, &ctx);
                    self.assign(&a.target, Some(value), &ctx);
                }
            }
            Stmt::AugAssign(a) => {
                self.scan(&a.value, &ctx);
                self.scan_target(&a.target, &ctx);
                // `obj.f += v`: the stored value is not derivable here.
                self.assign(&a.target, None, &ctx);
            }
            Stmt::Return(r) => {
                self.observations.push((None, narrowed.clone()));
                // A non-Optional return annotation is a requirement on every
                // returned value (`f -> f.<return>`), which callers rely on.
                let contract = self.func.has_return_contract();
                let site = r.range().start().to_u32();
                match &r.value {
                    Some(value) => {
                        self.return_call = match crate::value_analysis::strip_await(value) {
                            Expr::Call(c) => Some(c as *const _),
                            _ => None,
                        };
                        if contract {
                            self.emit(Target::Return, value, &ctx, false, site);
                        } else {
                            // The returned result of a call is consumed.
                            for (producer, call) in self.scope.producers_of(value, &ctx) {
                                self.use_site(&producer, call);
                            }
                        }
                        self.scan(value, &ctx);
                        self.return_call = None;
                    }
                    None if contract => self.none_return(site),
                    None => {}
                }
            }
            Stmt::Expr(e) => self.scan(&e.value, &ctx),
            Stmt::Raise(r) => {
                for e in r.exc.iter().chain(r.cause.iter()) {
                    self.scan(e, &ctx);
                }
            }
            Stmt::Assert(a) => {
                self.scan(&a.test, &ctx);
                if let Some(msg) = &a.msg {
                    self.scan(msg, &ctx.narrow(flow::negative(&a.test)));
                }
            }
            Stmt::Delete(d) => {
                for t in &d.targets {
                    self.scan_target(t, &ctx);
                }
            }
            _ => {}
        }
    }

    fn header(&mut self, expr: &'a Expr, narrowed: &Narrowed) {
        self.scan(expr, &Ctx::new(narrowed));
    }
}

/// Whether `class` is a dataclass or attrs class with its own `__init__`.
fn own_init(class: &ClassInfo) -> bool {
    use crate::dataclass_extractor::DataClassKind;
    matches!(
        class.kind,
        crate::resolve::ClassKind::Data(DataClassKind::Dataclass | DataClassKind::Attrs)
    ) && class.methods.contains_key("__init__")
}

/// Immediate child expressions of an expression.
struct Children<'a>(Vec<&'a Expr>);

impl<'a> Visitor<'a> for Children<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        self.0.push(expr);
    }
}

impl<'a, 's> EdgeWalker<'a, 's> {
    fn line_at(&self, offset: u32) -> u32 {
        self.lines.line(offset)
    }

    fn line_of(&self, expr: &Expr) -> u32 {
        self.line_at(expr.range().start().to_u32())
    }

    fn site_key(&self, call: &ast::ExprCall, callee: &str) -> SiteKey {
        SiteKey {
            file: self.func.source_file.clone(),
            offset: call.range().start().to_u32(),
            callee: callee.to_string(),
        }
    }

    /// Mark the result of `call` (a call of `callee`) as consumed: it gets a
    /// call-site node. Returns its key.
    fn use_site(&mut self, callee: &str, call: &ast::ExprCall) -> Option<SiteKey> {
        if !self.call_sites {
            return None;
        }
        let key = self.site_key(call, callee);
        let line = self.line_at(key.offset);
        self.used.entry(key.clone()).or_insert_with(|| CallSite {
            key: key.clone(),
            callee: callee.to_string(),
            line,
        });
        Some(key)
    }

    /// Find and handle every call in `expr`.
    fn scan(&mut self, expr: &'a Expr, ctx: &Ctx) {
        match expr {
            Expr::Call(call) => {
                if let Some(receiver) = self.saved_receiver(call) {
                    self.observations.push((Some(receiver), ctx.narrowed.clone()));
                }
                self.handle_call(call, ctx);
                self.scan(&call.func, ctx);
                for arg in call.arguments.args.iter() {
                    self.scan(arg, ctx);
                }
                for kw in call.arguments.keywords.iter() {
                    self.scan(&kw.value, ctx);
                }
            }
            Expr::If(i) => {
                self.scan(&i.test, ctx);
                self.scan(&i.body, &ctx.narrow(flow::positive(&i.test)));
                self.scan(&i.orelse, &ctx.narrow(flow::negative(&i.test)));
            }
            Expr::BoolOp(b) => {
                let mut c = ctx.clone();
                for v in &b.values {
                    self.scan(v, &c);
                    c = match b.op {
                        BoolOp::And => c.narrow(flow::positive(v)),
                        BoolOp::Or => c.narrow(flow::negative(v)),
                    };
                }
            }
            Expr::Lambda(l) => {
                let names: Vec<String> = l
                    .parameters
                    .as_ref()
                    .map(|p| p.iter().map(|p| p.name().to_string()).collect())
                    .unwrap_or_default();
                self.scan(&l.body, &ctx.shadow(names));
            }
            Expr::ListComp(_) | Expr::SetComp(_) | Expr::Generator(_) | Expr::DictComp(_) => {
                // Comprehension variables shadow the function's names.
                let c = self.scope.comprehension_ctx(expr, ctx);
                let mut children = Children(Vec::new());
                visitor::walk_expr(&mut children, expr);
                for child in children.0 {
                    self.scan(child, &c);
                }
            }
            _ => {
                let mut children = Children(Vec::new());
                visitor::walk_expr(&mut children, expr);
                for child in children.0 {
                    self.scan(child, ctx);
                }
            }
        }
    }

    /// Scan the evaluated parts of an assignment target (`f(x).attr = ...`).
    fn scan_target(&mut self, target: &'a Expr, ctx: &Ctx) {
        match target {
            Expr::Attribute(a) => self.scan(&a.value, ctx),
            Expr::Subscript(s) => {
                self.scan(&s.value, ctx);
                self.scan(&s.slice, ctx);
            }
            Expr::Tuple(t) => t.elts.iter().for_each(|e| self.scan_target(e, ctx)),
            Expr::List(l) => l.elts.iter().for_each(|e| self.scan_target(e, ctx)),
            Expr::Starred(s) => self.scan_target(&s.value, ctx),
            _ => {}
        }
    }

    /// The variable whose instance `call` saves: `obj.save(...)`, or
    /// `super().save(...)` (the method's `self`).
    fn saved_receiver(&self, call: &ast::ExprCall) -> Option<String> {
        let Expr::Attribute(attr) = call.func.as_ref() else {
            return None;
        };
        if !matches!(attr.attr.as_str(), "save" | "asave") {
            return None;
        }
        match attr.value.as_ref() {
            Expr::Name(n) => Some(n.id.to_string()),
            Expr::Call(c) if matches!(c.func.as_ref(), Expr::Name(n) if n.id.as_str() == "super") => {
                self.func.self_name().map(str::to_string)
            }
            _ => None,
        }
    }

    /// Record `obj.f = value` (a field of a known class) for
    /// `flush_attribute_writes`.
    fn defer_write(&mut self, target: &'a Expr, name: String, index: usize, value: &'a Expr, ctx: &Ctx) {
        let Expr::Attribute(attr) = target else { return };
        let Expr::Name(receiver) = attr.value.as_ref() else { return };
        let Some((class, true)) = self.scope.receiver_class(&attr.value, ctx) else {
            return;
        };
        if !class.kind.has_fields() {
            return;
        }
        let field = attr.attr.as_str();
        let Some(field) = class.fields.iter().find(|f| *f == field) else {
            return;
        };
        self.pending.push(PendingWrite {
            name,
            index,
            receiver: receiver.id.to_string(),
            class,
            field,
            value,
            ctx: ctx.clone(),
            site: target.range().start().to_u32(),
        });
    }

    /// Emit the field writes that are observed: the values of `obj.f` that
    /// reach an `obj.save()` (or, when the function never saves `obj`, one of
    /// its exits), each non-None when `obj.f` is narrowed there. A value
    /// overwritten before every observation is not written.
    fn flush_attribute_writes(&mut self) {
        let pending = std::mem::take(&mut self.pending);
        // (name, index) -> non-None at every observation it reaches
        let mut reached: HashMap<(String, usize), bool> = HashMap::new();
        let mut done: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for w in &pending {
            if !done.insert(w.name.as_str()) {
                continue;
            }
            let saves: Vec<&Narrowed> = self
                .observations
                .iter()
                .filter(|(r, _)| r.as_deref() == Some(w.receiver.as_str()))
                .map(|(_, n)| n)
                .collect();
            let points: Vec<&Narrowed> = if saves.is_empty() {
                self.observations
                    .iter()
                    .filter(|(r, _)| r.is_none())
                    .map(|(_, n)| n)
                    .collect()
            } else {
                saves
            };
            let count = self.attr_counts.get(&w.name).copied().unwrap_or(0);
            for n in points {
                let narrowed = n.names().any(|x| *x == w.name);
                let mut mark = |i: usize, nn: bool| {
                    let e = reached.entry((w.name.clone(), i)).or_insert(true);
                    *e &= nn || narrowed;
                };
                match n.defs(&w.name) {
                    Some(defs) => {
                        for &(d, nn) in defs {
                            if let flow::Def::Assign(i) = d {
                                mark(i, nn);
                            }
                        }
                    }
                    // Not known here (rebound in a loop, joined with a path
                    // that does not assign it): any assignment may reach.
                    None => (0..count).for_each(|i| mark(i, false)),
                }
            }
        }
        for w in pending {
            let Some(&non_none) = reached.get(&(w.name.clone(), w.index)) else {
                continue;
            };
            self.emit_value(Target::Field(w.class, w.field), w.value, &w.ctx, false, w.site, non_none);
        }
    }

    /// Attribute assignment `obj.f = value` to a field of a known class.
    /// `value` is `None` when the stored value is not derivable.
    fn assign(&mut self, target: &'a Expr, value: Option<&'a Expr>, ctx: &Ctx) {
        match target {
            Expr::Attribute(attr) => {
                let Some((class, true)) = self.scope.receiver_class(&attr.value, ctx) else {
                    return;
                };
                let field = attr.attr.as_str();
                if !class.kind.has_fields() || !class.fields.iter().any(|f| f == field) {
                    return;
                }
                let site = target.range().start().to_u32();
                match value {
                    Some(v) => self.emit(Target::Field(class, field), v, ctx, false, site),
                    None => self.push(
                        &self.func.qualified_name.clone(),
                        None,
                        Target::Field(class, field),
                        Some(Vec::new()),
                        site,
                    ),
                }
            }
            Expr::Tuple(t) => {
                let values = match value {
                    Some(Expr::Tuple(v))
                        if v.elts.len() == t.elts.len()
                            && !v.elts.iter().any(|e| matches!(e, Expr::Starred(_))) =>
                    {
                        Some(&v.elts)
                    }
                    _ => None,
                };
                for (i, elt) in t.elts.iter().enumerate() {
                    self.assign(elt, values.map(|v| &v[i]), ctx);
                }
            }
            Expr::List(l) => l.elts.iter().for_each(|e| self.assign(e, None, ctx)),
            Expr::Starred(s) => self.assign(&s.value, None, ctx),
            _ => {}
        }
    }

    fn handle_call(&mut self, call: &'a ast::ExprCall, ctx: &Ctx) {
        let is_return = self.return_call == Some(call as *const _);
        if let Some((class, method)) = self.scope.objects_call(call, ctx) {
            // Model.objects.create(f=v), .filter(...).update(f=v),
            // .get_or_create(f=v, defaults={...}), .update_or_create(...)
            if WRITE_METHODS.contains(&method) {
                self.keyword_writes(class, call, ctx, is_return);
            }
            return;
        }
        if self.special_writes(call, ctx, is_return) {
            return;
        }
        let callees = self.scope.callees(&call.func, ctx);
        let callee = match callees.as_slice() {
            [one] => one.clone(),
            targets => {
                // A method call that dispatches to several overrides.
                for t in targets {
                    if let Callee::Function { qualified, implicit } = t {
                        self.function_call(qualified, *implicit, call, ctx, true);
                    }
                }
                return;
            }
        };
        match callee {
            // A dataclass / attrs class that defines `__init__` gets no
            // generated constructor: the call binds `__init__`'s parameters.
            Callee::Class(class) if class.kind.has_fields() && !own_init(class) => {
                // Positional arguments bind the constructor's positional
                // parameters, when their order is known (dataclass, attrs, NamedTuple).
                let site = call.range().start().to_u32();
                for (i, arg) in call.arguments.args.iter().enumerate() {
                    if matches!(arg, Expr::Starred(_)) {
                        break;
                    }
                    let param = class.positional.as_ref().and_then(|p| p.get(i));
                    if let Some(field) = param.and_then(|p| class.fields.iter().find(|f| *f == p)) {
                        self.emit(Target::Field(class, field), arg, ctx, is_return, site);
                    }
                }
                self.keyword_writes(class, call, ctx, is_return);
            }
            Callee::Class(class) => {
                if let Some(init) = self.project.index.method(&class.qualified, "__init__") {
                    let implicit = self
                        .project
                        .index
                        .function(&init)
                        .map_or(0, |f| f.implicit_params());
                    // The call's value is the instance, not `__init__`'s result.
                    self.function_call(&init, implicit, call, ctx, false);
                }
            }
            Callee::Function {
                qualified,
                implicit,
            } => self.function_call(&qualified, implicit, call, ctx, true),
            Callee::Unknown => {}
        }
    }

    /// Writes that are not constructor calls: `dataclasses.replace(obj, f=v)`,
    /// `obj.model_copy(update={...})`, `Cls.model_validate({...})`,
    /// `setattr(obj, "f", v)`. Returns whether `call` was one.
    fn special_writes(&mut self, call: &'a ast::ExprCall, ctx: &Ctx, is_return: bool) -> bool {
        let args = &call.arguments.args;
        let site = call.range().start().to_u32();
        let func = call.func.as_ref();
        let external = |name: &str| self.scope.is_external(name, ctx);
        let parts = crate::resolve::dotted_parts(func).unwrap_or_default();
        let parts: Vec<&str> = parts.iter().map(String::as_str).collect();
        match parts.as_slice() {
            // replace(obj, f=v) / dataclasses.replace(obj, f=v)
            ["replace"] | ["dataclasses", "replace"]
                if external(parts[0]) && args.len() == 1 =>
            {
                let Some(class) = self.instance_class(&args[0], ctx) else { return false };
                self.keyword_writes(class, call, ctx, is_return);
                true
            }
            // setattr(obj, "f", v)
            ["setattr"] if external("setattr") && args.len() == 3 => {
                let Some(class) = self.instance_class(&args[0], ctx) else { return false };
                let Expr::StringLiteral(name) = &args[1] else {
                    // A computed field name: any field may be written.
                    self.dynamic_writes(class, &[], site);
                    return true;
                };
                let field = name.value.to_str();
                if let Some(field) = class.fields.iter().find(|f| *f == field) {
                    self.emit(Target::Field(class, field), &args[2], ctx, false, site);
                }
                true
            }
            _ => {
                let Expr::Attribute(attr) = func else { return false };
                // obj.__dict__.update(f=v, **d) / obj.__dict__.update(d)
                if attr.attr.as_str() == "update" {
                    if let Expr::Attribute(inner) = attr.value.as_ref() {
                        if inner.attr.as_str() == "__dict__" {
                            let Some(class) = self.instance_class(&inner.value, ctx) else {
                                return false;
                            };
                            self.keyword_writes(class, call, ctx, false);
                            match args.first() {
                                Some(d) if args.len() == 1 => self.splat_writes(class, d, ctx, false, site, true),
                                None => {}
                                Some(_) => self.dynamic_writes(class, &[], site),
                            }
                            return true;
                        }
                    }
                }
                match attr.attr.as_str() {
                    // obj.model_copy(update={...}) (pydantic v2; v1 `obj.copy(update=...)`)
                    "model_copy" | "copy" => {
                        let Some(class) = self.instance_class(&attr.value, ctx) else {
                            return false;
                        };
                        let pydantic = class.kind
                            == crate::resolve::ClassKind::Data(crate::dataclass_extractor::DataClassKind::Pydantic);
                        if !pydantic {
                            return false;
                        }
                        let update = call.arguments.keywords.iter().find(|k| k.arg.as_deref() == Some("update"));
                        if let Some(update) = update {
                            // Not validated: an unknown dict may write any field.
                            self.splat_writes(class, &update.value, ctx, is_return, site, true);
                        }
                        true
                    }
                    // Cls.model_validate({...}) (v1 `Cls.parse_obj({...})`)
                    "model_validate" | "parse_obj" if args.len() == 1 => {
                        let Some(class) = self.named_class(&attr.value, ctx) else { return false };
                        // Validation of input data: only the known entries are writes.
                        self.splat_writes(class, &args[0], ctx, is_return, site, false);
                        true
                    }
                    _ => false,
                }
            }
        }
    }

    /// Entries of a splatted dict (display, local, `dict(...)`, a returned
    /// dict) written to fields of `class`. With `dynamic`, a dict whose keys
    /// are not all known may write any other field: an edge without
    /// guarantees to each (a warning where the field has a requirement).
    /// For this function's own forwarded parameter (its entries are written
    /// at each call, see `kw_forwards`) only when nothing in the project
    /// calls the function.
    fn splat_writes(
        &mut self,
        class: &'a ClassInfo,
        dict: &'a Expr,
        ctx: &Ctx,
        is_return: bool,
        site: u32,
        dynamic: bool,
    ) {
        let splat = self.scope.splat(dict, ctx);
        let mut written: Vec<String> = Vec::new();
        for (key, value) in &splat.entries {
            if let Some(field) = class.fields.iter().find(|f| *f == key) {
                written.push(key.clone());
                self.emit_splat_value(Target::Field(class, field), value, is_return, site);
            }
        }
        if dynamic && !splat.complete {
            if self.forwarded_param(dict) {
                let before = self.edges.len();
                self.dynamic_writes(class, &written, site);
                let deferred: Vec<DiscoveredEdge> = self.edges.drain(before..).collect();
                self.forwarder_edges.extend(deferred);
            } else {
                self.dynamic_writes(class, &written, site);
            }
        }
    }

    /// Whether `expr` names a parameter of this function whose entries are
    /// forwarded (`**kw`, or a dict parameter it only splats).
    fn forwarded_param(&self, expr: &Expr) -> bool {
        let Expr::Name(n) = expr else { return false };
        let name = n.id.as_str();
        let Some(p) = self.func.params.iter().find(|p| p.name == name) else { return false };
        let key = match p.kind {
            ParamKind::VarKeywords => self.func.qualified_name.clone(),
            _ => dict_param_key(&self.func.qualified_name, name),
        };
        self.forwards.contains_key(&key)
    }

    /// A write whose fields are not known (`setattr(obj, name, v)`,
    /// `Cls(**d)` with an unknown `d`): an edge with no guarantees to every
    /// field of `class` other than `except`.
    fn dynamic_writes(&mut self, class: &'a ClassInfo, except: &[String], site: u32) {
        let source = self.func.qualified_name.clone();
        for field in &class.fields {
            if !except.contains(field) {
                self.push(&source, None, Target::Field(class, field), Some(Vec::new()), site);
            }
        }
    }

    /// Emit one known entry of a splatted dict into `target`.
    fn emit_splat_value(&mut self, target: Target<'a>, value: &SplatValue<'a>, is_return: bool, site: u32) {
        match value {
            SplatValue::Expr(e, c) => self.emit(target, e, c, is_return, site),
            SplatValue::Facts(f, file, offset) => {
                let line = self.project.lines_of(file).map_or(0, |l| l.line(*offset));
                self.emit_facts(target, f.clone(), file, line, is_return, site);
            }
        }
    }

    /// A value returned as `None`: a bare `return` (or falling off the end)
    /// under a non-Optional return annotation.
    fn none_return(&mut self, site: u32) {
        let line = self.line_at(site);
        let file = self.func.source_file.clone();
        self.emit_facts(Target::Return, ValueFacts::none_value(), &file, line, false, site);
    }

    /// The class (with fields) of the instance `expr` names.
    fn instance_class(&self, expr: &Expr, ctx: &Ctx) -> Option<&'a ClassInfo> {
        match self.scope.receiver_class(expr, ctx) {
            Some((class, true)) if class.kind.has_fields() => Some(class),
            _ => None,
        }
    }

    /// The class (with fields) `expr` names (`Invoice`, `cls` in a classmethod).
    fn named_class(&self, expr: &Expr, ctx: &Ctx) -> Option<&'a ClassInfo> {
        if let Some((class, false)) = self.scope.receiver_class(expr, ctx) {
            return class.kind.has_fields().then_some(class);
        }
        let parts = crate::resolve::dotted_parts(expr)?;
        if self.scope.is_local(&parts[0], ctx) {
            return None;
        }
        match self.project.index.resolve_dotted(&self.func.module, &parts)? {
            crate::resolve::Symbol::Class(q) => {
                self.project.index.class(&q).filter(|c| c.kind.has_fields())
            }
            _ => None,
        }
    }

    /// Keyword arguments (and `**{...}` expansions, and the `defaults=` /
    /// `create_defaults=` dicts of `get_or_create` / `update_or_create`) of a
    /// constructor-like call.
    fn keyword_writes(
        &mut self,
        class: &'a ClassInfo,
        call: &'a ast::ExprCall,
        ctx: &Ctx,
        is_return: bool,
    ) {
        let site = call.range().start().to_u32();
        let defaults_dicts = matches!(
            call.func.as_ref(),
            Expr::Attribute(a) if matches!(
                a.attr.as_str(),
                "get_or_create" | "update_or_create" | "aget_or_create" | "aupdate_or_create"
            )
        );
        let has = |f: &str| class.fields.iter().find(|x| *x == f);
        for kw in call.arguments.keywords.iter() {
            match &kw.arg {
                Some(name)
                    if defaults_dicts
                        && matches!(name.as_str(), "defaults" | "create_defaults")
                        && has(name.as_str()).is_none() =>
                {
                    self.splat_writes(class, &kw.value, ctx, is_return, site, true);
                }
                Some(name) => {
                    if let Some(field) = has(name.as_str()) {
                        self.emit(Target::Field(class, field), &kw.value, ctx, is_return, site);
                    }
                }
                None => self.splat_writes(class, &kw.value, ctx, is_return, site, true),
            }
        }
    }

    /// A call to an extracted function: a `calls` edge, and `flows_to`
    /// edges per argument bound to a named parameter (into the callee's own
    /// node, and into the call-site node when `result` and the result is
    /// consumed). Keywords that land in a forwarded `**kw` become writes.
    fn function_call(
        &mut self,
        callee_q: &str,
        implicit: usize,
        call: &'a ast::ExprCall,
        ctx: &Ctx,
        result: bool,
    ) {
        let Some(callee) = self.project.index.function(callee_q) else {
            return;
        };
        let callee: &'a FunctionInfo = callee;
        let site = call.range().start().to_u32();
        let mut calls = DiscoveredEdge::new(&self.func.qualified_name, callee_q, None, "calls");
        calls.site = Some((self.func.source_file.clone(), self.line_at(site)));
        self.edges.push(calls);
        let call_site = (result && self.call_sites).then(|| self.site_key(call, callee_q));
        let Some(params) = callee.params.get(implicit..) else {
            return;
        };
        // An explicit `self` (`Cls.method(obj, ...)`) is not a value parameter.
        let self_param = if implicit == 0 {
            callee.self_name()
        } else {
            None
        };
        let positional: Vec<&'a str> = params
            .iter()
            .take_while(|p| matches!(p.kind, ParamKind::PositionalOnly | ParamKind::Normal))
            .map(|p| p.name.as_str())
            .collect();
        let by_keyword = |name: &str| {
            params
                .iter()
                .find(|p| {
                    p.name == name && matches!(p.kind, ParamKind::Normal | ParamKind::KeywordOnly)
                })
                .map(|p| p.name.as_str())
                .filter(|p| Some(*p) != self_param)
        };
        for (i, arg) in call.arguments.args.iter().enumerate() {
            if matches!(arg, Expr::Starred(_)) {
                break; // later positions are unknown
            }
            if let Some(param) = positional.get(i).filter(|p| Some(**p) != self_param) {
                self.emit_flow(callee, param, arg, ctx, &call_site, site);
                self.forward_dict(callee_q, param, arg, ctx, site);
            }
        }
        // Keywords that no named parameter takes, for `**kw` forwarding.
        let mut extra: Vec<(String, SplatValue<'a>)> = Vec::new();
        let mut unknown_extra = false;
        for kw in call.arguments.keywords.iter() {
            match &kw.arg {
                Some(name) => match by_keyword(name.as_str()) {
                    Some(param) => {
                        self.emit_flow(callee, param, &kw.value, ctx, &call_site, site);
                        self.forward_dict(callee_q, param, &kw.value, ctx, site);
                    }
                    None => extra.push((name.to_string(), SplatValue::Expr(&kw.value, ctx.clone()))),
                },
                None => {
                    let splat = self.scope.splat(&kw.value, ctx);
                    unknown_extra |= !splat.complete && !self.forwarded_param(&kw.value);
                    for (key, value) in splat.entries {
                        match by_keyword(&key) {
                            Some(param) => self.emit_flow_value(callee, param, &value, &call_site, site),
                            None => extra.push((key, value)),
                        }
                    }
                }
            }
        }
        if !extra.is_empty() || unknown_extra {
            self.forward(callee_q, &extra, !unknown_extra, site, 0);
        }
    }

    /// A dict passed as parameter `param` of a dict forwarder: its entries
    /// are written where the callee splats it (a dict with unknown keys may
    /// write any field there).
    fn forward_dict(&mut self, callee_q: &str, param: &str, arg: &'a Expr, ctx: &Ctx, site: u32) {
        let key = dict_param_key(callee_q, param);
        if !self.forwards.contains_key(&key) {
            return;
        }
        let splat = self.scope.splat(arg, ctx);
        let complete = splat.complete || self.forwarded_param(arg);
        self.forward(&key, &splat.entries, complete, site, 0);
    }

    /// Keywords passed to `callee`'s `**kw` (or a dict passed to a dict
    /// forwarder), followed to where it forwards them. Unless `complete`,
    /// other keys may be present: any other field of a target class may be
    /// written (`dynamic_writes`).
    fn forward(
        &mut self,
        callee_q: &str,
        extra: &[(String, SplatValue<'a>)],
        complete: bool,
        site: u32,
        depth: usize,
    ) {
        if depth > 4 {
            return;
        }
        let Some(targets) = self.forwards.get(callee_q) else {
            return;
        };
        for target in targets.clone() {
            match target {
                Forward::Class(q) => {
                    let Some(class) = self.project.index.class(&q) else { continue };
                    let mut written = Vec::new();
                    for (key, value) in extra {
                        if let Some(field) = class.fields.iter().find(|f| *f == key) {
                            written.push(key.clone());
                            self.emit_splat_value(Target::Field(class, field), value, false, site);
                        }
                    }
                    if !complete {
                        self.dynamic_writes(class, &written, site);
                    }
                }
                Forward::Function(g) => {
                    let Some(gf) = self.project.index.function(&g) else { continue };
                    let gf: &'a FunctionInfo = gf;
                    let mut rest = Vec::new();
                    for (key, value) in extra {
                        let param = gf.named_value_params().find(|p| {
                            &p.name == key
                                && matches!(p.kind, ParamKind::Normal | ParamKind::KeywordOnly)
                        });
                        match param {
                            Some(p) => self.emit_flow_value(gf, &p.name, value, &None, site),
                            None => rest.push((key.clone(), value.clone())),
                        }
                    }
                    if !rest.is_empty() || !complete {
                        self.forward(&g, &rest, complete, site, depth + 1);
                    }
                }
            }
        }
    }

    /// `value` binds `param` of `callee`: into its own node and its call-site node.
    fn emit_flow(
        &mut self,
        callee: &'a FunctionInfo,
        param: &'a str,
        value: &'a Expr,
        ctx: &Ctx,
        call_site: &Option<SiteKey>,
        site: u32,
    ) {
        let q = callee.qualified_name.as_str();
        self.emit(Target::Function(q, param, None), value, ctx, false, site);
        if let Some(key) = call_site {
            self.emit(Target::Function(q, param, Some(key.clone())), value, ctx, false, site);
        }
    }

    /// `emit_flow` for an entry of a splatted dict.
    fn emit_flow_value(
        &mut self,
        callee: &'a FunctionInfo,
        param: &'a str,
        value: &SplatValue<'a>,
        call_site: &Option<SiteKey>,
        site: u32,
    ) {
        let q = callee.qualified_name.as_str();
        self.emit_splat_value(Target::Function(q, param, None), value, false, site);
        if let Some(key) = call_site {
            self.emit_splat_value(Target::Function(q, param, Some(key.clone())), value, false, site);
        }
    }

    /// Emit the edge carrying `value` into `target`; `site` is the offset of
    /// the write or call expression.
    fn emit(&mut self, target: Target<'a>, value: &'a Expr, ctx: &Ctx, is_return: bool, site: u32) {
        self.emit_value(target, value, ctx, is_return, site, false);
    }

    /// `emit`; with `non_none`, the value is known not to be None where it is
    /// observed. A value with a few alternatives (`a if c else b`, the
    /// reaching definitions of a local) gets one edge per alternative, all at
    /// the same site, so an alternative that certainly violates a
    /// requirement is an error even when another one is unknown.
    fn emit_value(
        &mut self,
        target: Target<'a>,
        value: &'a Expr,
        ctx: &Ctx,
        is_return: bool,
        site: u32,
        non_none: bool,
    ) {
        let line = self.line_of(value);
        for alt in self.scope.alternatives(value, ctx) {
            match alt {
                Alt::Expr(e, c, nn) => {
                    self.emit_expr(target.clone(), e, &c, is_return, site, non_none || nn)
                }
                Alt::Facts(f) => {
                    let f = if non_none { f.non_none_part() } else { f };
                    let file = self.func.source_file.clone();
                    self.emit_facts(target.clone(), f, &file, line, is_return, site);
                }
            }
        }
    }

    /// One alternative `value` (evaluated with `ctx`) into `target`: from the
    /// producer's call-site node when it is the result of an extracted
    /// function (one edge per dispatch target), else from the enclosing
    /// function with the value's own contracts.
    fn emit_expr(&mut self, target: Target<'a>, value: &'a Expr, ctx: &Ctx, is_return: bool, site: u32, non_none: bool) {
        let producers = self.scope.producers_of(value, ctx);
        if !producers.is_empty() {
            // A local narrowed to non-None here: the producer's postconditions
            // with nullability replaced.
            let narrowed = non_none
                || matches!(value, Expr::Name(n) if ctx.narrowed.contains(n.id.as_str()));
            for (producer, call) in producers {
                let key = self.use_site(&producer, call);
                let rows = (narrowed
                    && self
                        .project
                        .summaries
                        .get(&producer)
                        .and_then(|s| s.nullable)
                        != Some(false))
                .then(|| self.narrowed_copy(&producer, value));
                if key.is_none() && matches!(&target, Target::Function(h, _, _) if *h == producer) {
                    continue; // `h(h(x))` without call-site nodes: no self-loop
                }
                self.push(&producer, key, target.clone(), rows, site);
            }
            return;
        }
        if matches!(&target, Target::Function(h, _, None) if *h == self.func.qualified_name) {
            return; // recursion: no self-loop on the function's own node
        }
        let facts = self.scope.facts(value, ctx);
        let facts = if non_none { facts.non_none_part() } else { facts };
        let file = self.func.source_file.clone();
        let line = self.line_of(value);
        self.emit_facts(target, facts, &file, line, is_return, site);
    }

    /// An edge from the enclosing function into `target` whose source
    /// guarantees are `facts` (located at `file:line`). `None` written into a
    /// field that accepts None meets every requirement of the field: no edge.
    fn emit_facts(&mut self, target: Target<'a>, facts: ValueFacts, file: &str, line: u32, is_return: bool, site: u32) {
        if facts.always_none {
            if let Target::Field(class, field) = &target {
                if class.field_nullable.get(*field) == Some(&true) {
                    return;
                }
            }
        }
        let mut rows = facts_rows(&facts, 0, file, line, VerificationLevel::Extracted);
        if is_return && matches!(target, Target::Field(..)) {
            self.add_docstring_rows(&mut rows);
        }
        self.push(&self.func.qualified_name.clone(), None, target, Some(rows), site);
    }

    /// F's docstring `ensures:` rows (ASSUMED) for kinds the extraction did not
    /// determine; a static docstring precision is also added next to a bound
    /// that depends on F's input.
    fn add_docstring_rows(&self, rows: &mut Vec<ContractRecord>) {
        let extracted = rows.clone();
        for doc in &self.doc_rows {
            let same: Vec<&ContractRecord> = extracted
                .iter()
                .filter(|r| r.constraint_type == doc.constraint_type)
                .collect();
            let add = same.is_empty()
                || (doc.constraint_type == ConstraintType::Precision
                    && doc.dependent_expr.is_none()
                    && same.iter().all(|r| r.dependent_expr.is_some()));
            if add {
                rows.push(doc.clone());
            }
        }
    }

    /// The producer's node postconditions with nullability set to non-null.
    fn narrowed_copy(&self, producer: &str, value: &Expr) -> Vec<ContractRecord> {
        let Some(func) = self.project.index.function(producer) else {
            return Vec::new();
        };
        let summary = self
            .project
            .summaries
            .get(producer)
            .cloned()
            .unwrap_or_default();
        let doc = self
            .project
            .docstrings
            .get(producer)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let mut rows: Vec<ContractRecord> =
            function_extractor::postcondition_rows(func, &summary, doc, 0)
                .into_iter()
                .filter(|r| r.constraint_type != ConstraintType::Nullability)
                .collect();
        rows.push(ContractRecord {
            param_nullable: Some(0),
            ..ContractRecord::new(
                0,
                ConstraintType::Nullability,
                crate::db::ContractRole::Postcondition,
                VerificationLevel::Extracted,
                &self.func.source_file,
                self.line_of(value),
            )
        });
        rows
    }

    fn push(
        &mut self,
        source: &str,
        source_site: Option<SiteKey>,
        target: Target<'a>,
        rows: Option<Vec<ContractRecord>>,
        site: u32,
    ) {
        let mut edge = match target {
            Target::Field(class, field) => {
                DiscoveredEdge::new(source, &class.qualified, Some(field), "writes_to")
            }
            Target::Function(h, param, target_site) => DiscoveredEdge {
                target_param: Some(param.to_string()),
                target_site,
                ..DiscoveredEdge::new(source, h, None, "flows_to")
            },
            Target::Return => {
                DiscoveredEdge::new(source, &self.func.return_node_name(), None, "writes_to")
            }
        };
        edge.source_site = source_site;
        edge.site = Some((self.func.source_file.clone(), self.line_at(site)));
        edge.override_rows = rows;
        self.edges.push(edge);
    }
}

// --- Override loading ---

/// Override edge from TOML config.
#[derive(Debug, Deserialize)]
pub struct OverrideEdge {
    pub source: String,
    pub target: String,
    pub relationship: String,
}

/// Override config file structure.
#[derive(Debug, Deserialize)]
pub struct OverrideConfig {
    #[serde(default)]
    pub edges: Vec<OverrideEdge>,
}

/// Load manual edge overrides from a TOML file.
pub fn load_overrides(path: &std::path::Path) -> anyhow::Result<Vec<DiscoveredEdge>> {
    let content = std::fs::read_to_string(path)?;
    let config: OverrideConfig = toml::from_str(&content)?;

    Ok(config
        .edges
        .into_iter()
        .map(|e| DiscoveredEdge {
            discovery: "manual".to_string(),
            ..DiscoveredEdge::new(&e.source, &e.target, None, &e.relationship)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edges(files: &[(&str, &str)]) -> Vec<DiscoveredEdge> {
        let project = Project::from_sources(
            files
                .iter()
                .map(|(p, s)| (p.to_string(), s.to_string()))
                .collect(),
        );
        discover_project_edges(&project)
    }

    /// `(source, target[.field], relationship, target_param, override?)`
    fn summary(es: &[DiscoveredEdge]) -> Vec<(String, String, String, Option<String>, bool)> {
        let mut v: Vec<_> = es
            .iter()
            .map(|e| {
                let target = match &e.target_field {
                    Some(f) => format!("{}.{f}", e.target_name),
                    None => e.target_name.clone(),
                };
                (
                    e.source_function.clone(),
                    target,
                    e.relationship.clone(),
                    e.target_param.clone(),
                    e.override_rows.is_some(),
                )
            })
            .collect();
        v.sort();
        v
    }

    fn row_kinds(e: &DiscoveredEdge) -> Vec<String> {
        let mut kinds: Vec<String> = e
            .override_rows
            .as_ref()
            .unwrap()
            .iter()
            .map(|r| {
                let v = r
                    .param_decimal_places
                    .map(|p| p.to_string())
                    .or(r.param_nullable.map(|n| n.to_string()))
                    .or(r.param_type_name.clone())
                    .or(r.param_max_length.map(|n| n.to_string()))
                    .or(r.param_choices.clone())
                    .or(r.dependent_expr.clone())
                    .unwrap_or_default();
                format!("{}={v}", r.constraint_type.as_str())
            })
            .collect();
        kinds.sort();
        kinds
    }

    const RECORDS: &str = "from pydantic import BaseModel, Field\nclass Invoice(BaseModel):\n    total: Decimal = Field(decimal_places=2)\n    tax: Decimal\n";

    fn owned(
        v: &[(&str, &str, &str, Option<&str>, bool)],
    ) -> Vec<(String, String, String, Option<String>, bool)> {
        let mut v: Vec<_> = v
            .iter()
            .map(|(a, b, c, d, e)| {
                (
                    a.to_string(),
                    b.to_string(),
                    c.to_string(),
                    d.map(str::to_string),
                    *e,
                )
            })
            .collect();
        v.sort();
        v
    }

    #[test]
    fn test_writes_producer_vs_override() {
        let es = edges(&[
            ("records.py", RECORDS),
            (
                "code.py",
                "from records import Invoice\ndef g(a):\n    return a\ndef make(a, b):\n    x = g(a)\n    return Invoice(total=x, tax=b.quantize(Decimal('0.1')))\n",
            ),
        ]);
        assert_eq!(
            summary(&es),
            owned(&[
                ("code.g", "records.Invoice.total", "writes_to", None, false),
                ("code.make", "records.Invoice.tax", "writes_to", None, true),
                ("code.make", "code.g", "calls", None, false),
                // into g's own node and into the call site `g(a)` (its result is written)
                ("code.make", "code.g", "flows_to", Some("a"), true),
                ("code.make", "code.g", "flows_to", Some("a"), true),
            ])
        );
        let tax = es
            .iter()
            .find(|e| e.target_field.as_deref() == Some("tax"))
            .unwrap();
        assert_eq!(
            row_kinds(tax),
            ["nullability=0", "precision=1", "type=Decimal"]
        );
    }

    #[test]
    fn test_kwargs_dicts_positional_and_attribute_writes() {
        let es = edges(&[
            ("records.py", "from dataclasses import dataclass\n@dataclass\nclass P:\n    a: int\n    b: str\n"),
            (
                "code.py",
                "from records import P\ndef k():\n    d = {'a': 1, 'b': 'x'}\n    return P(**d)\ndef lit():\n    return P(**{'a': 2})\ndef pos():\n    return P(3, 'y')\ndef attr(p: P):\n    p.b = None\ndef local():\n    p = P(1, 'z')\n    p.a = 5\ndef mutated():\n    d = {'a': 1}\n    d['b'] = 'q'\n    return P(**d)\n",
            ),
        ]);
        let targets = |src: &str| {
            let mut v: Vec<String> = es
                .iter()
                .filter(|e| e.source_function == src && e.relationship == "writes_to")
                .map(|e| e.target_field.clone().unwrap())
                .collect();
            v.sort();
            v
        };
        assert_eq!(targets("code.k"), ["a", "b"]);
        assert_eq!(targets("code.lit"), ["a"]);
        assert_eq!(targets("code.pos"), ["a", "b"]);
        assert_eq!(targets("code.attr"), ["b"]);
        assert_eq!(targets("code.local"), ["a", "a", "b"]);
        // A mutated dict is not expanded: its keys are unknown, so every
        // field gets an edge without guarantees (a dynamic write).
        assert_eq!(targets("code.mutated"), ["a", "b"]);
        assert!(es
            .iter()
            .filter(|e| e.source_function == "code.mutated" && e.relationship == "writes_to")
            .all(|e| e.override_rows.as_ref().is_some_and(|r| r.is_empty())));
        let none_write = es
            .iter()
            .find(|e| e.source_function == "code.attr")
            .unwrap();
        // `None`: nullable, and within any precision, length or choices bound.
        assert_eq!(
            row_kinds(none_write),
            ["choices=[]", "length=0", "nullability=1", "precision=0"]
        );
    }

    #[test]
    fn test_positional_writes_skip_private_parameters() {
        let es = edges(&[
            ("records.py", "from dataclasses import dataclass, field\n@dataclass\nclass P:\n    a: int\n    _b: int\n    c: str\n@dataclass\nclass Q:\n    a: int\n    b: int = field(init=False)\n    c: str = ''\n"),
            ("code.py", "from records import P, Q\ndef f():\n    P(1, 2, 'x')\n    Q(1, 'y')\n"),
        ]);
        let mut fields: Vec<String> = es
            .iter()
            .filter(|e| e.relationship == "writes_to")
            .map(|e| format!("{}.{}", e.target_name, e.target_field.as_deref().unwrap()))
            .collect();
        fields.sort();
        // P: `_b` is a constructor parameter (not an extracted field), so 'x' binds `c`.
        // Q: `init=False` makes the order uncertain: no positional writes.
        assert_eq!(fields, ["records.P.a", "records.P.c"]);
    }

    #[test]
    fn test_flows_to_target_params() {
        let es = edges(&[(
            "code.py",
            "def h(a, b=0, *args, c=1, **kw):\n    pass\ndef g():\n    return 1\ndef f(x):\n    h(g(), x, 5, c=g(), zz=x)\n    h(*x, b=1)\n",
        )]);
        let flows: Vec<(String, Option<String>, bool)> = {
            let mut v: Vec<_> = es
                .iter()
                .filter(|e| e.relationship == "flows_to")
                .map(|e| {
                    (
                        e.source_function.clone(),
                        e.target_param.clone(),
                        e.override_rows.is_some(),
                    )
                })
                .collect();
            v.sort();
            v
        };
        assert_eq!(
            flows,
            [
                ("code.f".to_string(), Some("b".to_string()), true),
                ("code.f".to_string(), Some("b".to_string()), true),
                ("code.g".to_string(), Some("a".to_string()), false),
                ("code.g".to_string(), Some("c".to_string()), false),
            ]
        );
    }

    #[test]
    fn test_methods_and_django() {
        let es = edges(&[
            (
                "models.py",
                "from django.db import models\nclass R(models.Model):\n    kwh = models.DecimalField(max_digits=5, decimal_places=3)\n    def fix(self, v):\n        self.kwh = v\n",
            ),
            (
                "code.py",
                "from models import R\nclass Pricer:\n    def rounded(self, a):\n        return a\n    def make(self, a):\n        return R.objects.create(kwh=self.rounded(a))\ndef load(pk):\n    r = R.objects.get(pk=pk)\n    r.kwh = 1\n    R.objects.filter(pk=pk).update(kwh=2)\n",
            ),
        ]);
        let s = summary(&es);
        for want in [
            (
                "code.Pricer.rounded",
                "models.R.kwh",
                "writes_to",
                None,
                false,
            ),
            (
                "code.Pricer.make",
                "code.Pricer.rounded",
                "flows_to",
                Some("a"),
                true,
            ),
            ("code.load", "models.R.kwh", "writes_to", None, true),
            ("models.R.fix", "models.R.kwh", "writes_to", None, true),
        ] {
            let want = owned(&[want]).remove(0);
            assert!(s.contains(&want), "missing {want:?} in {s:?}");
        }
        assert_eq!(
            s.iter()
                .filter(|e| e.0 == "code.load" && e.2 == "writes_to")
                .count(),
            2
        );
    }

    #[test]
    fn test_docstring_ensures_on_returned_write() {
        let es = edges(&[
            ("models.py", "from django.db import models\nclass R(models.Model):\n    kwh = models.DecimalField(max_digits=5, decimal_places=3)\n"),
            (
                "code.py",
                "from models import R\ndef rec(v):\n    \"\"\"ensures: precision(result) <= 5\"\"\"\n    return R.objects.create(kwh=v)\ndef other(v):\n    \"\"\"ensures: precision(result) <= 5\"\"\"\n    R.objects.create(kwh=v)\n",
            ),
        ]);
        let rows = |src: &str| {
            let e = es
                .iter()
                .find(|e| e.source_function == src && e.relationship == "writes_to")
                .unwrap();
            row_kinds(e)
        };
        // `v` depends on rec's only input; the static docstring bound is added.
        assert_eq!(
            rows("code.rec"),
            ["precision=5", "precision=input_precision"]
        );
        // Not the returned call: docstrings describe the return value only.
        assert_eq!(rows("code.other"), ["precision=input_precision"]);
    }

    #[test]
    fn test_narrowed_producer_local_copies_postconditions() {
        let es = edges(&[
            ("records.py", RECORDS),
            (
                "code.py",
                "from records import Invoice\nP = {}\ndef find(k):\n    return P.get(k)\ndef make(k):\n    p = find(k)\n    if p is None:\n        raise ValueError()\n    return Invoice(total=p)\ndef unsafe(k):\n    p = find(k)\n    return Invoice(total=p)\n",
            ),
        ]);
        let writes: Vec<&DiscoveredEdge> = es
            .iter()
            .filter(|e| e.relationship == "writes_to")
            .collect();
        assert_eq!(writes.len(), 2);
        assert!(writes.iter().all(|e| e.source_function == "code.find"));
        let narrowed = writes
            .iter()
            .find(|e| e.override_rows.is_some())
            .expect("narrowed copy");
        assert_eq!(row_kinds(narrowed), ["nullability=0"]);
        assert_eq!(
            writes.iter().filter(|e| e.override_rows.is_none()).count(),
            1
        );
    }

    #[test]
    fn test_function_local_import_and_unbound_method() {
        let es = edges(&[
            (
                "helpers.py",
                "def label(a):\n    return a\nclass C:\n    def m(self, v):\n        return v\n",
            ),
            (
                "code.py",
                "def f(x, c):\n    from helpers import label, C\n    label(x)\n    C.m(c, x)\n",
            ),
        ]);
        let flows: Vec<(String, Option<String>)> = {
            let mut v: Vec<_> = es
                .iter()
                .filter(|e| e.relationship == "flows_to")
                .map(|e| (e.target_name.clone(), e.target_param.clone()))
                .collect();
            v.sort();
            v
        };
        assert_eq!(
            flows,
            [
                ("helpers.C.m".to_string(), Some("v".to_string())),
                ("helpers.label".to_string(), Some("a".to_string())),
            ]
        );
    }

    #[test]
    fn test_unresolved_calls_give_no_edges() {
        let es = edges(&[("code.py", "import helpers\ndef f(x, obj):\n    len(x)\n    helpers.nothing(x)\n    obj.method(x)\n    Unknown(a=x)\n")]);
        assert!(es.is_empty(), "{es:?}");
    }

    fn discovered(files: &[(&str, &str)]) -> Discovered {
        let project = Project::from_sources(
            files
                .iter()
                .map(|(p, s)| (p.to_string(), s.to_string()))
                .collect(),
        );
        discover(&project)
    }

    /// `(source[@line], target[@line][.field], relationship, param)` for
    /// every non-`calls` edge, sorted; `@line` marks a call-site node.
    fn site_summary(d: &Discovered) -> Vec<String> {
        let line = |k: &Option<SiteKey>| {
            k.as_ref()
                .map(|k| {
                    let l = d.call_sites.iter().find(|s| &s.key == k).map(|s| s.line);
                    format!("@{}", l.unwrap_or(0))
                })
                .unwrap_or_default()
        };
        let mut v: Vec<String> = d
            .edges
            .iter()
            .filter(|e| e.relationship != "calls")
            .map(|e| {
                let target = match &e.target_field {
                    Some(f) => format!("{}.{f}", e.target_name),
                    None => format!("{}{}", e.target_name, line(&e.target_site)),
                };
                format!(
                    "{}{} -{}-> {}{}",
                    e.source_function,
                    line(&e.source_site),
                    e.relationship,
                    target,
                    e.target_param.as_ref().map(|p| format!(" [{p}]")).unwrap_or_default()
                )
            })
            .collect();
        v.sort();
        v
    }

    const INV: &str = "from pydantic import BaseModel, Field\nclass Inv(BaseModel):\n    total: Decimal = Field(decimal_places=2)\n    wide: Decimal = Field(decimal_places=4)\n";

    /// Round 3 F2 / M3: each consumed call's result is its own node, so two
    /// call sites of one helper are not joined, and every write is located.
    #[test]
    fn test_call_site_nodes() {
        let d = discovered(&[
            ("records.py", INV),
            (
                "code.py",
                "from records import Inv\n\
                 def keep(p):\n    return p\n\
                 def four(x):\n    return x\n\
                 def two(x):\n    return x\n\
                 def make_wide(x):\n    return Inv(total=Decimal('0'), wide=keep(four(x)))\n\
                 def make_total(x):\n    return Inv(total=keep(two(x)), wide=Decimal('0'))\n\
                 def unused(x):\n    four(x)\n",
            ),
        ]);
        assert_eq!(
            site_summary(&d),
            [
                "code.four@9 -flows_to-> code.keep [p]",
                "code.four@9 -flows_to-> code.keep@9 [p]",
                "code.keep@11 -writes_to-> records.Inv.total",
                "code.keep@9 -writes_to-> records.Inv.wide",
                "code.make_total -flows_to-> code.two [x]",
                "code.make_total -flows_to-> code.two@11 [x]",
                "code.make_total -writes_to-> records.Inv.wide",
                "code.make_wide -flows_to-> code.four [x]",
                "code.make_wide -flows_to-> code.four@9 [x]",
                "code.make_wide -writes_to-> records.Inv.total",
                "code.two@11 -flows_to-> code.keep [p]",
                "code.two@11 -flows_to-> code.keep@11 [p]",
                // `four(x)` as a statement: its result is not consumed, no call site.
                "code.unused -flows_to-> code.four [x]",
            ]
        );
        // Every consumed call site, located at the call.
        let mut sites: Vec<(String, u32)> =
            d.call_sites.iter().map(|s| (s.callee.clone(), s.line)).collect();
        sites.sort();
        assert_eq!(
            sites,
            [
                ("code.four".to_string(), 9),
                ("code.keep".to_string(), 9),
                ("code.keep".to_string(), 11),
                ("code.two".to_string(), 11),
            ]
        );
        // Edge sites: the write or call expression.
        let w = d
            .edges
            .iter()
            .find(|e| e.target_field.as_deref() == Some("wide") && e.source_site.is_some())
            .unwrap();
        assert_eq!(w.site, Some(("code.py".to_string(), 9)));
    }

    #[test]
    fn test_returned_and_local_producers_get_sites() {
        let d = discovered(&[
            ("records.py", INV),
            (
                "code.py",
                "from records import Inv\n\
                 def g(a):\n    return a\n\
                 def f(a):\n    return g(a)\n\
                 def h(a):\n    x = g(a)\n    y = g(a)\n    return Inv(total=x, wide=y)\n",
            ),
        ]);
        let mut sites: Vec<(String, u32)> =
            d.call_sites.iter().map(|s| (s.callee.clone(), s.line)).collect();
        sites.sort();
        assert_eq!(
            sites,
            [("code.g".to_string(), 5), ("code.g".to_string(), 7), ("code.g".to_string(), 8)]
        );
        let s = site_summary(&d);
        assert!(s.contains(&"code.g@7 -writes_to-> records.Inv.total".to_string()), "{s:?}");
        assert!(s.contains(&"code.g@8 -writes_to-> records.Inv.wide".to_string()), "{s:?}");
    }

    /// Round 3 D6: module-level code is the pseudo function `<module m>`.
    #[test]
    fn test_module_level_code() {
        let d = discovered(&[
            ("records.py", INV),
            (
                "code.py",
                "from records import Inv\ndef four(x):\n    return x\nDEFAULT = Inv(total=four(Decimal('1')))\nOTHER = Inv(wide=Decimal('1.5'))\n",
            ),
        ]);
        let s = site_summary(&d);
        assert!(s.contains(&"code.four@4 -writes_to-> records.Inv.total".to_string()), "{s:?}");
        assert!(s.contains(&"<module code> -writes_to-> records.Inv.wide".to_string()), "{s:?}");
        assert!(s.contains(&"<module code> -flows_to-> code.four [x]".to_string()), "{s:?}");
    }

    /// Round 3 D7: `**kw` forwarders and manager write methods.
    #[test]
    fn test_kwargs_forwarding_and_manager_writes() {
        let d = discovered(&[
            ("records.py", INV),
            (
                "models.py",
                "from django.db import models\nclass Price(models.Model):\n    value = models.DecimalField(max_digits=10, decimal_places=2)\n    name = models.CharField(max_length=5)\n",
            ),
            (
                "code.py",
                "from records import Inv\nfrom models import Price\n\
                 def four(x):\n    return x\n\
                 def build(**kw):\n    return Inv(**kw)\n\
                 def create_price(**fields):\n    return Price.objects.create(**fields)\n\
                 def outer(**kw):\n    return build(**kw)\n\
                 def mutating(**kw):\n    kw.pop('total')\n    return Inv(**kw)\n\
                 def a(x):\n    build(total=four(x))\n\
                 def b(x):\n    create_price(value=four(x))\n\
                 def c(x):\n    outer(wide=four(x))\n\
                 def m(x):\n    mutating(total=four(x))\n\
                 def goc(x):\n    Price.objects.update_or_create(pk=1, defaults={'value': four(x)})\n\
                 def lookups(x):\n    Price.objects.get_or_create(value=four(x), name='n')\n",
            ),
        ]);
        let s = site_summary(&d);
        for want in [
            "code.four@15 -writes_to-> records.Inv.total",
            "code.four@17 -writes_to-> models.Price.value",
            "code.four@19 -writes_to-> records.Inv.wide",
            "code.four@23 -writes_to-> models.Price.value",
            "code.four@25 -writes_to-> models.Price.value",
            "code.lookups -writes_to-> models.Price.name",
        ] {
            assert!(s.contains(&want.to_string()), "missing {want} in {s:#?}");
        }
        // A forwarder that mutates its **kw is not followed.
        assert!(!s.iter().any(|e| e.starts_with("code.four@21 -writes_to")), "{s:#?}");
        // `pk` is not a field: no write.
        assert!(!s.iter().any(|e| e.contains("Price.pk")));
    }

    /// `(target field, nullability of the override rows)` of every write
    /// from `source` (override edges only).
    fn writes_from(d: &Discovered, source: &str) -> Vec<(String, Option<i64>)> {
        let mut v: Vec<(String, Option<i64>)> = d
            .edges
            .iter()
            .filter(|e| e.relationship == "writes_to" && e.source_function == source)
            .map(|e| {
                let n = e.override_rows.as_ref().and_then(|rows| {
                    rows.iter()
                        .find(|r| r.constraint_type == ConstraintType::Nullability)
                        .and_then(|r| r.param_nullable)
                });
                (format!("{}.{}", e.target_name, e.target_field.clone().unwrap()), n)
            })
            .collect();
        v.sort();
        v
    }

    /// Round 5 N2: `obj.f = v` is written as the values of `obj.f` that reach
    /// the save (or the exit when there is none), narrowed there.
    #[test]
    fn test_flow_sensitive_attribute_writes() {
        let d = discovered(&[(
            "models.py",
            "from django.db import models\n\
             class C(models.Model):\n    t = models.CharField(max_length=5)\n\
             \x20   def a(self, d):\n        self.t = d.get('k')\n        if self.t is None:\n            self.t = 'x'\n        self.save()\n\
             \x20   def b(self, d):\n        self.t = d.get('k')\n        self.save()\n\
             \x20   def c(self):\n        self.t = None\n        self.t = 'y'\n        self.save()\n\
             \x20   def e(self, d):\n        self.t = d.get('k')\n        self.save()\n        self.t = None\n\
             \x20   def g(self, d, xs):\n        for x in xs:\n            self.t = d.get(x)\n        self.save()\n\
             \x20   def h(self, d):\n        self.t = d.get('k')\n        if not self.t:\n            raise ValueError()\n        super().save()\n\
             def nosave(c: C, d):\n    c.t = d.get('k')\n    if c.t is None:\n        return\n\
             def other(c: C, k: C, d):\n    c.t = d.get('k')\n    k.save()\n",
        )]);
        // a: the lookup narrowed non-None at the save, and 'x'
        assert_eq!(writes_from(&d, "models.C.a"), [("models.C.t".into(), Some(0)), ("models.C.t".into(), Some(0))]);
        assert_eq!(writes_from(&d, "models.C.b"), [("models.C.t".into(), Some(1))]);
        // c: `None` is overwritten before the save
        assert_eq!(writes_from(&d, "models.C.c"), [("models.C.t".into(), Some(0))]);
        // e: the None after the save is not saved here
        assert_eq!(writes_from(&d, "models.C.e"), [("models.C.t".into(), Some(1))]);
        // g: rebound in a loop: every assignment
        assert_eq!(writes_from(&d, "models.C.g"), [("models.C.t".into(), Some(1))]);
        // h: `super().save()` observes self
        assert_eq!(writes_from(&d, "models.C.h"), [("models.C.t".into(), Some(0))]);
        // nosave: observed at the exits (the early return sees the None)
        assert_eq!(writes_from(&d, "models.nosave"), [("models.C.t".into(), Some(1))]);
        // other: `k.save()` does not save `c`: observed at the exit
        assert_eq!(writes_from(&d, "models.other"), [("models.C.t".into(), Some(1))]);
    }

    /// Round 5 N4 / N7 / N12: `cls(...)`, awaited producers and async ORM
    /// writes, `replace` / `model_copy` / `model_validate` / `setattr`, dict
    /// forwarders, dict spreads.
    #[test]
    fn test_round5_write_patterns() {
        let d = discovered(&[
            ("records.py", INV),
            (
                "models.py",
                "from django.db import models\nclass M(models.Model):\n    v = models.DecimalField(max_digits=5, decimal_places=2)\n",
            ),
            (
                "code.py",
                "import dataclasses\nfrom records import Inv\nfrom models import M\n\
                 def four(x):\n    return x\n\
                 async def afour(x):\n    return x\n\
                 class Ser:\n    def create(self, data):\n        return M.objects.create(**data)\n\
                 class Inv2(Inv):\n    @classmethod\n    def make(cls, x):\n        return cls(total=four(x))\n\
                 async def a(x):\n    return await M.objects.acreate(v=await afour(x))\n\
                 def b(i: Inv, x):\n    return i.model_copy(update={'wide': four(x)})\n\
                 def c(x):\n    return Inv.model_validate({'total': four(x)})\n\
                 def e(i: Inv, x):\n    setattr(i, 'wide', four(x))\n\
                 def f(x):\n    return Ser().create({'v': x}), Ser.create(Ser(), {'v': four(x)})\n\
                 def g(x, other):\n    return Inv(**{'total': four(x), **other, 'wide': four(x)})\n\
                 def h(i: Inv, x):\n    return dataclasses.replace(i, total=four(x))\n",
            ),
        ]);
        let s = site_summary(&d);
        for want in [
            "code.four@14 -writes_to-> code.Inv2.total",
            "code.afour@16 -writes_to-> models.M.v",
            "code.four@18 -writes_to-> records.Inv.wide",
            "code.four@20 -writes_to-> records.Inv.total",
            "code.four@22 -writes_to-> records.Inv.wide",
            "code.four@24 -writes_to-> models.M.v",
            "code.four@26 -writes_to-> records.Inv.wide",
            "code.four@28 -writes_to-> records.Inv.total",
        ] {
            assert!(s.contains(&want.to_string()), "missing {want} in {s:#?}");
        }
        // `{'total': ..., **other}`: `other` may replace total (a write
        // without guarantees).
        assert!(
            d.edges.iter().filter(|e| e.target_field.as_deref() == Some("total")
                && e.site.as_ref().is_some_and(|s| s.1 == 26)).all(|e| e.override_rows.as_ref().is_some_and(|r| r.is_empty())),
            "{s:#?}"
        );
    }

    /// A dataclass with its own `__init__` binds that method's parameters,
    /// and a generic base (`Base[T]`) is resolved to its class.
    #[test]
    fn test_own_init_and_generic_bases() {
        let d = discovered(&[(
            "code.py",
            "from dataclasses import dataclass\n\
             @dataclass\nclass Base[N]:\n    node: object\n\
             @dataclass\nclass Ctx(Base[N]):\n    flag: bool = True\n    def __init__(self, node, flag: bool = True):\n        self.node = node\n        self.flag = flag\n\
             @dataclass\nclass Plain(Base[N]):\n    extra: str = ''\n\
             def f(x):\n    return Ctx(x), Plain(node=x, extra=None)\n",
        )]);
        let s = site_summary(&d);
        assert!(s.contains(&"code.f -flows_to-> code.Ctx.__init__ [node]".to_string()), "{s:#?}");
        assert!(!s.iter().any(|e| e.contains("Ctx.flag") && e.starts_with("code.f ")), "{s:#?}");
        // Plain inherits `node` through the generic base.
        assert!(s.contains(&"code.f -writes_to-> code.Plain.node".to_string()), "{s:#?}");
    }
}
