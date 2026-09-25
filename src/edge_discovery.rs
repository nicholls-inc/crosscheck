use ruff_python_ast::{self as ast, Expr, Stmt};
use serde::Deserialize;
use std::collections::HashMap;

/// A discovered edge between two nodes in the contract graph.
#[derive(Debug)]
pub struct DiscoveredEdge {
    pub source_function: String,
    pub target_name: String,
    pub target_field: Option<String>,
    pub relationship: String,
    pub discovery: String,
}

/// Discover edges from ORM/constructor write patterns and function calls in function bodies.
///
/// Relationships:
/// - `writes_to`: a function writes a field, via `Model.objects.create(field=...)` or a
///   constructor call `Cls(field=...)` / `Cls(a, b)`. When the written value is the result
///   of another function (`Cls(field=g(...))`, or `x = g(...)` then `Cls(field=x)`), a
///   `writes_to` edge from `g` to the field is emitted as well.
/// - `calls`: a function calls another function (caller → callee).
/// - `flows_to`: the result of one function is passed as an argument to another
///   (`h(g(...))`, or `x = g(...)` then `h(x)`), edge `g → h`.
pub fn discover_edges(stmts: &[Stmt]) -> Vec<DiscoveredEdge> {
    let mut edges = Vec::new();

    for stmt in stmts {
        match stmt {
            Stmt::FunctionDef(func_def) => discover_edges_in_function(func_def, &mut edges),
            Stmt::ClassDef(class_def) => {
                for body_stmt in &class_def.body {
                    if let Stmt::FunctionDef(func_def) = body_stmt {
                        discover_edges_in_function(func_def, &mut edges);
                    }
                }
            }
            _ => {}
        }
    }

    edges
}

/// Per-function discovery context.
struct Ctx<'a> {
    func_name: &'a str,
    /// Local variables bound exactly once to the result of a function call: name → callee.
    call_bindings: HashMap<String, String>,
}

fn discover_edges_in_function(func_def: &ast::StmtFunctionDef, edges: &mut Vec<DiscoveredEdge>) {
    let func_name = func_def.name.to_string();
    let mut bindings: HashMap<String, Option<String>> = HashMap::new();
    collect_call_bindings(&func_def.body, &mut bindings);
    let ctx = Ctx {
        func_name: &func_name,
        call_bindings: bindings
            .into_iter()
            .filter_map(|(name, callee)| callee.map(|c| (name, c)))
            .collect(),
    };
    discover_edges_in_body(&func_def.body, &ctx, edges);
}

/// Record `name = callee(...)` bindings. A name assigned more than once, or
/// assigned anything other than a direct function call, maps to `None`.
fn collect_call_bindings(stmts: &[Stmt], out: &mut HashMap<String, Option<String>>) {
    for stmt in stmts {
        match stmt {
            Stmt::Assign(assign) => {
                for target in &assign.targets {
                    if let Expr::Name(n) = target {
                        let callee = match assign.value.as_ref() {
                            Expr::Call(call) => is_function_call(&call.func),
                            _ => None,
                        };
                        bind(out, &n.id, callee);
                    }
                }
            }
            Stmt::AnnAssign(ann) => {
                if let Expr::Name(n) = ann.target.as_ref() {
                    let callee = match ann.value.as_deref() {
                        Some(Expr::Call(call)) => is_function_call(&call.func),
                        _ => None,
                    };
                    bind(out, &n.id, callee);
                }
            }
            Stmt::AugAssign(aug) => {
                if let Expr::Name(n) = aug.target.as_ref() {
                    bind(out, &n.id, None);
                }
            }
            Stmt::For(for_stmt) => {
                if let Expr::Name(n) = for_stmt.target.as_ref() {
                    bind(out, &n.id, None);
                }
                collect_call_bindings(&for_stmt.body, out);
            }
            Stmt::If(if_stmt) => {
                collect_call_bindings(&if_stmt.body, out);
                for clause in &if_stmt.elif_else_clauses {
                    collect_call_bindings(&clause.body, out);
                }
            }
            Stmt::While(w) => collect_call_bindings(&w.body, out),
            Stmt::With(w) => collect_call_bindings(&w.body, out),
            Stmt::Try(t) => {
                collect_call_bindings(&t.body, out);
                for handler in &t.handlers {
                    let ast::ExceptHandler::ExceptHandler(h) = handler;
                    collect_call_bindings(&h.body, out);
                }
                collect_call_bindings(&t.orelse, out);
                collect_call_bindings(&t.finalbody, out);
            }
            _ => {}
        }
    }
}

fn bind(out: &mut HashMap<String, Option<String>>, name: &str, callee: Option<String>) {
    out.entry(name.to_string())
        .and_modify(|existing| *existing = None)
        .or_insert(callee);
}

/// The function whose result `expr` is, if it is a direct call or a bound local.
fn value_source(expr: &Expr, ctx: &Ctx) -> Option<String> {
    match expr {
        Expr::Call(call) => is_function_call(&call.func),
        Expr::Name(n) => ctx.call_bindings.get(n.id.as_str()).cloned(),
        _ => None,
    }
}

fn edge(source: &str, target: &str, field: Option<String>, relationship: &str) -> DiscoveredEdge {
    DiscoveredEdge {
        source_function: source.to_string(),
        target_name: target.to_string(),
        target_field: field,
        relationship: relationship.to_string(),
        discovery: "ast_pattern".to_string(),
    }
}

/// Discover edges within a function body.
fn discover_edges_in_body(stmts: &[Stmt], ctx: &Ctx, edges: &mut Vec<DiscoveredEdge>) {
    for stmt in stmts {
        discover_edges_in_stmt(stmt, ctx, edges);
    }
}

/// Discover edges in a single statement.
fn discover_edges_in_stmt(stmt: &Stmt, ctx: &Ctx, edges: &mut Vec<DiscoveredEdge>) {
    match stmt {
        Stmt::Expr(expr_stmt) => {
            discover_edges_in_expr(&expr_stmt.value, ctx, edges);
        }
        Stmt::Assign(assign) => {
            discover_edges_in_expr(&assign.value, ctx, edges);
        }
        Stmt::AnnAssign(ann_assign) => {
            if let Some(value) = &ann_assign.value {
                discover_edges_in_expr(value, ctx, edges);
            }
        }
        Stmt::Return(ret) => {
            if let Some(value) = &ret.value {
                discover_edges_in_expr(value, ctx, edges);
            }
        }
        Stmt::If(if_stmt) => {
            discover_edges_in_body(&if_stmt.body, ctx, edges);
            for clause in &if_stmt.elif_else_clauses {
                discover_edges_in_body(&clause.body, ctx, edges);
            }
        }
        Stmt::For(for_stmt) => {
            discover_edges_in_body(&for_stmt.body, ctx, edges);
        }
        Stmt::While(while_stmt) => {
            discover_edges_in_body(&while_stmt.body, ctx, edges);
        }
        Stmt::With(with_stmt) => {
            discover_edges_in_body(&with_stmt.body, ctx, edges);
        }
        Stmt::Try(try_stmt) => {
            discover_edges_in_body(&try_stmt.body, ctx, edges);
            for handler in &try_stmt.handlers {
                let ast::ExceptHandler::ExceptHandler(h) = handler;
                discover_edges_in_body(&h.body, ctx, edges);
            }
        }
        _ => {}
    }
}

/// Emit `writes_to` edges for one written field: from the enclosing function,
/// and from the function that produced the value, if any.
fn push_write(
    edges: &mut Vec<DiscoveredEdge>,
    ctx: &Ctx,
    model_name: &str,
    field: String,
    value: &Expr,
) {
    if let Some(producer) = value_source(value, ctx) {
        edges.push(edge(&producer, model_name, Some(field.clone()), "writes_to"));
    }
    edges.push(edge(ctx.func_name, model_name, Some(field), "writes_to"));
}

/// Discover edges in an expression.
fn discover_edges_in_expr(expr: &Expr, ctx: &Ctx, edges: &mut Vec<DiscoveredEdge>) {
    match expr {
        // Pattern 1: Model.objects.create(field=expr)
        Expr::Call(call) => {
            if let Some(model_name) = is_objects_create(&call.func) {
                for keyword in &call.arguments.keywords {
                    if let Some(field_name) = &keyword.arg {
                        push_write(edges, ctx, &model_name, field_name.to_string(), &keyword.value);
                    }
                }
            }
            // Pattern 2: Model(field=expr) -- constructor call.
            // Positional arguments are emitted with a `#<index>` placeholder
            // field; the extractor maps them to declared fields for data
            // classes whose constructors take fields in order.
            else if let Some(model_name) = is_model_constructor(&call.func) {
                for (index, arg) in call.arguments.args.iter().enumerate() {
                    if matches!(arg, Expr::Starred(_)) {
                        break;
                    }
                    push_write(edges, ctx, &model_name, format!("#{index}"), arg);
                }
                for keyword in &call.arguments.keywords {
                    if let Some(field_name) = &keyword.arg {
                        push_write(edges, ctx, &model_name, field_name.to_string(), &keyword.value);
                    }
                }
            }
            // Pattern 3: function_call(args) -- function-to-function call,
            // plus data flow from any argument that is another function's result.
            else if let Some(callee_name) = is_function_call(&call.func) {
                edges.push(edge(ctx.func_name, &callee_name, None, "calls"));
                let values = call
                    .arguments
                    .args
                    .iter()
                    .chain(call.arguments.keywords.iter().map(|k| &k.value));
                for value in values {
                    if let Some(producer) = value_source(value, ctx) {
                        if producer != callee_name {
                            edges.push(edge(&producer, &callee_name, None, "flows_to"));
                        }
                    }
                }
            }
            // Recurse into call arguments
            for arg in &call.arguments.args {
                discover_edges_in_expr(arg, ctx, edges);
            }
            for keyword in &call.arguments.keywords {
                discover_edges_in_expr(&keyword.value, ctx, edges);
            }
        }
        _ => {}
    }
}

/// Check if expression is Model.objects.create.
fn is_objects_create(expr: &Expr) -> Option<String> {
    if let Expr::Attribute(attr) = expr {
        if attr.attr.as_str() == "create" {
            if let Expr::Attribute(inner) = attr.value.as_ref() {
                if inner.attr.as_str() == "objects" {
                    if let Expr::Name(name) = inner.value.as_ref() {
                        return Some(name.id.to_string());
                    }
                }
            }
        }
    }
    None
}

/// Check if expression is a Model constructor call (capitalized name).
fn is_model_constructor(expr: &Expr) -> Option<String> {
    if let Expr::Name(name) = expr {
        let first = name.id.chars().next()?;
        if first.is_uppercase() {
            return Some(name.id.to_string());
        }
    }
    None
}

/// Check if expression is a function call (lowercase or underscore name).
/// Complement of `is_model_constructor`: lowercase = function, uppercase = model.
/// Calls to builtins like `len()`, `print()` are emitted but silently dropped
/// during edge resolution when they don't match any extracted function name.
fn is_function_call(expr: &Expr) -> Option<String> {
    if let Expr::Name(name) = expr {
        let first = name.id.chars().next()?;
        if first.is_lowercase() || first == '_' {
            return Some(name.id.to_string());
        }
    }
    None
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
            source_function: e.source,
            target_name: e.target,
            target_field: None,
            relationship: e.relationship,
            discovery: "manual".to_string(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: parse a Python expression and return it.
    fn parse_expr(source: &str) -> Expr {
        let parsed = ruff_python_parser::parse_unchecked(
            source,
            ruff_python_parser::Mode::Module.into(),
        );
        match parsed.into_syntax() {
            ruff_python_ast::Mod::Module(module) => {
                if let Stmt::Expr(expr_stmt) = &module.body[0] {
                    (*expr_stmt.value).clone()
                } else {
                    panic!("expected expression statement");
                }
            }
            _ => panic!("expected module"),
        }
    }

    // -- is_objects_create tests --

    #[test]
    fn test_objects_create_match() {
        let expr = parse_expr("Model.objects.create(a=1)");
        if let Expr::Call(call) = &expr {
            assert_eq!(is_objects_create(&call.func), Some("Model".to_string()));
        } else {
            panic!("expected call");
        }
    }

    #[test]
    fn test_objects_filter_no_match() {
        let expr = parse_expr("Model.objects.filter(a=1)");
        if let Expr::Call(call) = &expr {
            assert_eq!(is_objects_create(&call.func), None);
        } else {
            panic!("expected call");
        }
    }

    #[test]
    fn test_plain_create_no_match() {
        // obj.create() without .objects. should not match
        let expr = parse_expr("obj.create(a=1)");
        if let Expr::Call(call) = &expr {
            assert_eq!(is_objects_create(&call.func), None);
        } else {
            panic!("expected call");
        }
    }

    // -- is_model_constructor tests --

    #[test]
    fn test_uppercase_is_model() {
        let expr = parse_expr("Model");
        assert_eq!(is_model_constructor(&expr), Some("Model".to_string()));
    }

    #[test]
    fn test_lowercase_not_model() {
        let expr = parse_expr("helper");
        assert_eq!(is_model_constructor(&expr), None);
    }

    #[test]
    fn test_underscore_not_model() {
        let expr = parse_expr("_Foo");
        assert_eq!(is_model_constructor(&expr), None);
    }

    // -- is_function_call tests --

    #[test]
    fn test_lowercase_is_function() {
        let expr = parse_expr("helper");
        assert_eq!(is_function_call(&expr), Some("helper".to_string()));
    }

    #[test]
    fn test_underscore_prefix_is_function() {
        let expr = parse_expr("_private");
        assert_eq!(is_function_call(&expr), Some("_private".to_string()));
    }

    #[test]
    fn test_uppercase_not_function() {
        let expr = parse_expr("Model");
        assert_eq!(is_function_call(&expr), None);
    }
}
