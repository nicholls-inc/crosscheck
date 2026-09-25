//! Edge discovery (data-flow model v2, `docs/design/dataflow-v2.md`).
//!
//! | Code | Edge |
//! |------|------|
//! | `Cls(f=g(...))`, `x = g(...); Cls(f=x)`, likewise `objects.create`, `**{...}`, positional, `obj.f = g(...)` | `g writes_to Cls.f` |
//! | `Cls(f=expr)` etc., `expr` not a call to an extracted function | `F writes_to Cls.f`, override = contracts of `expr` in F |
//! | `h(..., g(...), ...)`, or via a local bound to `g(...)` | `g flows_to h`, `target_param` = the parameter it binds |
//! | `h(..., expr, ...)`, `expr` not such a call | `F flows_to h`, `target_param` set, override = contracts of `expr` |
//! | `h(...)` | `F calls h` (structural, not checked) |
//!
//! `F` is the enclosing function. Names are resolved through `resolve`; a
//! call or class that does not resolve to something extracted gives no edge.

use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, BoolOp, Expr, Stmt};
use ruff_text_size::Ranged;
use serde::Deserialize;

use crate::db::{ConstraintType, ContractRecord, VerificationLevel};
use crate::extractor::Project;
use crate::flow::{self, FlowVisitor, FunctionFlow, Narrowed};
use crate::function_extractor::{self, FunctionInfo, ParamKind};
use crate::resolve::ClassInfo;
use crate::value_analysis::{facts_rows, Callee, Ctx, Scope};

/// A discovered edge between two nodes in the contract graph.
#[derive(Debug, Clone)]
pub struct DiscoveredEdge {
    /// Qualified name of the source function.
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
        }
    }
}

/// Discover edges in a single module, as the unnamed module `""` (so
/// qualified names are the bare names). Convenience for tests.
pub fn discover_edges(stmts: &[Stmt]) -> Vec<DiscoveredEdge> {
    let project = Project::from_parsed(vec![crate::extractor::SourceModule {
        relative_path: String::new(),
        module: String::new(),
        is_package: false,
        source: String::new(),
        stmts: stmts.to_vec(),
    }]);
    discover_project_edges(&project)
}

/// Discover the edges of every extracted function in the project.
pub fn discover_project_edges(project: &Project) -> Vec<DiscoveredEdge> {
    let mut edges = Vec::new();
    for func in &project.index.functions {
        let flow = FunctionFlow::of(&func.body);
        let scope = Scope::new(&project.index, &project.summaries, Some(func), &flow);
        let doc = project
            .docstrings
            .get(&func.qualified_name)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let mut walker = EdgeWalker {
            project,
            scope: &scope,
            func,
            source: project.source_of(&func.source_file),
            doc_rows: function_extractor::docstring_postcondition_rows(func, doc, 0),
            return_call: None,
            edges: Vec::new(),
        };
        flow::walk_block(&func.body, &Narrowed::new(), &mut walker);
        edges.extend(walker.edges);
    }
    edges
}

enum Target<'a> {
    Field(&'a ClassInfo, &'a str),
    /// A callee (qualified name) and the parameter the value binds.
    Function(&'a str, &'a str),
}

struct EdgeWalker<'a, 's> {
    project: &'a Project,
    scope: &'s Scope<'a>,
    func: &'a FunctionInfo,
    source: &'a str,
    /// The enclosing function's docstring `ensures:` rows (ASSUMED).
    doc_rows: Vec<ContractRecord>,
    /// The call that is the value of the `return` statement being scanned.
    return_call: Option<*const ast::ExprCall>,
    edges: Vec<DiscoveredEdge>,
}

impl<'a, 's> FlowVisitor<'a> for EdgeWalker<'a, 's> {
    fn simple(&mut self, stmt: &'a Stmt, narrowed: &Narrowed) {
        let ctx = Ctx::new(narrowed);
        match stmt {
            Stmt::Assign(a) => {
                self.scan(&a.value, &ctx);
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
                if let Some(value) = &r.value {
                    self.return_call = match value.as_ref() {
                        Expr::Call(c) => Some(c as *const _),
                        _ => None,
                    };
                    self.scan(value, &ctx);
                    self.return_call = None;
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

/// Immediate child expressions of an expression.
struct Children<'a>(Vec<&'a Expr>);

impl<'a> Visitor<'a> for Children<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        self.0.push(expr);
    }
}

fn target_names(expr: &Expr) -> Vec<String> {
    flow::bound_names_in_expr(expr).into_iter().collect()
}

impl<'a, 's> EdgeWalker<'a, 's> {
    fn line_of(&self, expr: &Expr) -> u32 {
        crate::source::line_of(self.source, expr.range().start().to_u32())
    }

    /// Find and handle every call in `expr`.
    fn scan(&mut self, expr: &'a Expr, ctx: &Ctx) {
        match expr {
            Expr::Call(call) => {
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
                let c = ctx.shadow(target_names(expr));
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
                match value {
                    Some(v) => self.emit(Target::Field(class, field), v, ctx, false),
                    None => self.push(
                        &self.func.qualified_name.clone(),
                        &Target::Field(class, field),
                        Some(Vec::new()),
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
            // Model.objects.create(f=v), Model.objects.filter(...).update(f=v)
            if method == "create" || method == "update" {
                self.keyword_writes(class, call, ctx, is_return);
            }
            return;
        }
        match self.scope.callee(&call.func, ctx) {
            Callee::Class(class) if class.kind.has_fields() => {
                // Positional arguments bind the constructor's positional
                // parameters, when their order is known (dataclass, attrs, NamedTuple).
                for (i, arg) in call.arguments.args.iter().enumerate() {
                    if matches!(arg, Expr::Starred(_)) {
                        break;
                    }
                    let param = class.positional.as_ref().and_then(|p| p.get(i));
                    if let Some(field) = param.and_then(|p| class.fields.iter().find(|f| *f == p)) {
                        self.emit(Target::Field(class, field), arg, ctx, is_return);
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
                    self.function_call(&init, implicit, call, ctx);
                }
            }
            Callee::Function {
                qualified,
                implicit,
            } => self.function_call(&qualified, implicit, call, ctx),
            Callee::Unknown => {}
        }
    }

    /// Keyword arguments (and `**{...}` expansions) of a constructor-like call.
    fn keyword_writes(
        &mut self,
        class: &'a ClassInfo,
        call: &'a ast::ExprCall,
        ctx: &Ctx,
        is_return: bool,
    ) {
        let has = |f: &str| class.fields.iter().find(|x| *x == f);
        for kw in call.arguments.keywords.iter() {
            match &kw.arg {
                Some(name) => {
                    if let Some(field) = has(name.as_str()) {
                        self.emit(Target::Field(class, field), &kw.value, ctx, is_return);
                    }
                }
                None => {
                    for (key, value, c) in self.scope.dict_items(&kw.value, ctx).unwrap_or_default()
                    {
                        if let Some(field) = has(&key) {
                            self.emit(Target::Field(class, field), value, &c, is_return);
                        }
                    }
                }
            }
        }
    }

    /// A call to an extracted function: a `calls` edge, and a `flows_to`
    /// edge per argument bound to a named parameter.
    fn function_call(
        &mut self,
        callee_q: &str,
        implicit: usize,
        call: &'a ast::ExprCall,
        ctx: &Ctx,
    ) {
        let Some(callee) = self.project.index.function(callee_q) else {
            return;
        };
        let callee: &'a FunctionInfo = callee;
        self.edges.push(DiscoveredEdge::new(
            &self.func.qualified_name,
            callee_q,
            None,
            "calls",
        ));
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
                self.emit_flow(callee, param, arg, ctx);
            }
        }
        for kw in call.arguments.keywords.iter() {
            match &kw.arg {
                Some(name) => {
                    if let Some(param) = by_keyword(name.as_str()) {
                        self.emit_flow(callee, param, &kw.value, ctx);
                    }
                }
                None => {
                    for (key, value, c) in self.scope.dict_items(&kw.value, ctx).unwrap_or_default()
                    {
                        if let Some(param) = by_keyword(&key) {
                            self.emit_flow(callee, param, value, &c);
                        }
                    }
                }
            }
        }
    }

    fn emit_flow(&mut self, callee: &'a FunctionInfo, param: &'a str, value: &'a Expr, ctx: &Ctx) {
        self.emit(
            Target::Function(&callee.qualified_name, param),
            value,
            ctx,
            false,
        );
    }

    /// Emit the edge carrying `value` into `target`.
    fn emit(&mut self, target: Target<'a>, value: &'a Expr, ctx: &Ctx, is_return: bool) {
        if let Some(producer) = self.scope.producer_of(value, ctx) {
            if matches!(target, Target::Function(h, _) if h == producer) {
                return; // `h(h(x))`: no self-loop
            }
            // A local narrowed to non-None here: the producer's postconditions
            // with nullability replaced.
            let narrowed = matches!(value, Expr::Name(n) if ctx.narrowed.contains(n.id.as_str()));
            let rows = (narrowed
                && self
                    .project
                    .summaries
                    .get(&producer)
                    .and_then(|s| s.nullable)
                    != Some(false))
            .then(|| self.narrowed_copy(&producer, value));
            self.push(&producer, &target, rows);
            return;
        }
        if matches!(target, Target::Function(h, _) if h == self.func.qualified_name) {
            return; // recursion: no self-loop
        }
        let facts = self.scope.facts(value, ctx);
        let mut rows = facts_rows(
            &facts,
            0,
            &self.func.source_file,
            self.line_of(value),
            VerificationLevel::Extracted,
        );
        if is_return && matches!(target, Target::Field(..)) {
            self.add_docstring_rows(&mut rows);
        }
        self.push(&self.func.qualified_name.clone(), &target, Some(rows));
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

    fn push(&mut self, source: &str, target: &Target<'a>, rows: Option<Vec<ContractRecord>>) {
        let mut edge = match target {
            Target::Field(class, field) => {
                DiscoveredEdge::new(source, &class.qualified, Some(field), "writes_to")
            }
            Target::Function(h, param) => DiscoveredEdge {
                target_param: Some(param.to_string()),
                ..DiscoveredEdge::new(source, h, None, "flows_to")
            },
        };
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
        assert!(
            targets("code.mutated").is_empty(),
            "a mutated dict is not expanded"
        );
        let none_write = es
            .iter()
            .find(|e| e.source_function == "code.attr")
            .unwrap();
        assert_eq!(row_kinds(none_write), ["nullability=1"]);
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
}
