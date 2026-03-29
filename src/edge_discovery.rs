use ruff_python_ast::{self as ast, Expr, Stmt};
use serde::Deserialize;

/// A discovered edge between two nodes in the contract graph.
#[derive(Debug)]
pub struct DiscoveredEdge {
    pub source_function: String,
    pub target_name: String,
    pub target_field: Option<String>,
    pub relationship: String,
    pub discovery: String,
}

/// Discover edges from Django ORM write patterns and function calls in function bodies.
pub fn discover_edges(stmts: &[Stmt]) -> Vec<DiscoveredEdge> {
    let mut edges = Vec::new();

    for stmt in stmts {
        match stmt {
            Stmt::FunctionDef(func_def) => {
                let func_name = func_def.name.to_string();
                discover_edges_in_body(&func_def.body, &func_name, &mut edges);
            }
            Stmt::ClassDef(class_def) => {
                for body_stmt in &class_def.body {
                    if let Stmt::FunctionDef(func_def) = body_stmt {
                        let func_name = func_def.name.to_string();
                        discover_edges_in_body(&func_def.body, &func_name, &mut edges);
                    }
                }
            }
            _ => {}
        }
    }

    edges
}

/// Discover edges within a function body.
fn discover_edges_in_body(stmts: &[Stmt], func_name: &str, edges: &mut Vec<DiscoveredEdge>) {
    for stmt in stmts {
        discover_edges_in_stmt(stmt, func_name, edges);
    }
}

/// Discover edges in a single statement.
fn discover_edges_in_stmt(stmt: &Stmt, func_name: &str, edges: &mut Vec<DiscoveredEdge>) {
    match stmt {
        Stmt::Expr(expr_stmt) => {
            discover_edges_in_expr(&expr_stmt.value, func_name, edges);
        }
        Stmt::Assign(assign) => {
            discover_edges_in_expr(&assign.value, func_name, edges);
        }
        Stmt::AnnAssign(ann_assign) => {
            if let Some(value) = &ann_assign.value {
                discover_edges_in_expr(value, func_name, edges);
            }
        }
        Stmt::Return(ret) => {
            if let Some(value) = &ret.value {
                discover_edges_in_expr(value, func_name, edges);
            }
        }
        Stmt::If(if_stmt) => {
            discover_edges_in_body(&if_stmt.body, func_name, edges);
            for clause in &if_stmt.elif_else_clauses {
                discover_edges_in_body(&clause.body, func_name, edges);
            }
        }
        Stmt::For(for_stmt) => {
            discover_edges_in_body(&for_stmt.body, func_name, edges);
        }
        Stmt::While(while_stmt) => {
            discover_edges_in_body(&while_stmt.body, func_name, edges);
        }
        Stmt::With(with_stmt) => {
            discover_edges_in_body(&with_stmt.body, func_name, edges);
        }
        Stmt::Try(try_stmt) => {
            discover_edges_in_body(&try_stmt.body, func_name, edges);
            for handler in &try_stmt.handlers {
                let ast::ExceptHandler::ExceptHandler(h) = handler;
                discover_edges_in_body(&h.body, func_name, edges);
            }
        }
        _ => {}
    }
}

/// Discover edges in an expression.
fn discover_edges_in_expr(expr: &Expr, func_name: &str, edges: &mut Vec<DiscoveredEdge>) {
    match expr {
        // Pattern 1: Model.objects.create(field=expr)
        Expr::Call(call) => {
            if let Some(model_name) = is_objects_create(&call.func) {
                for keyword in &call.arguments.keywords {
                    if let Some(field_name) = &keyword.arg {
                        edges.push(DiscoveredEdge {
                            source_function: func_name.to_string(),
                            target_name: model_name.clone(),
                            target_field: Some(field_name.to_string()),
                            relationship: "writes_to".to_string(),
                            discovery: "ast_pattern".to_string(),
                        });
                    }
                }
            }
            // Pattern 2: Model(field=expr) -- constructor call
            else if let Some(model_name) = is_model_constructor(&call.func) {
                for keyword in &call.arguments.keywords {
                    if let Some(field_name) = &keyword.arg {
                        edges.push(DiscoveredEdge {
                            source_function: func_name.to_string(),
                            target_name: model_name.clone(),
                            target_field: Some(field_name.to_string()),
                            relationship: "writes_to".to_string(),
                            discovery: "ast_pattern".to_string(),
                        });
                    }
                }
            }
            // Pattern 3: function_call(args) -- function-to-function call
            else if let Some(callee_name) = is_function_call(&call.func) {
                edges.push(DiscoveredEdge {
                    source_function: func_name.to_string(),
                    target_name: callee_name,
                    target_field: None,
                    relationship: "calls".to_string(),
                    discovery: "ast_pattern".to_string(),
                });
            }
            // Recurse into call arguments
            for arg in &call.arguments.args {
                discover_edges_in_expr(arg, func_name, edges);
            }
            for keyword in &call.arguments.keywords {
                discover_edges_in_expr(&keyword.value, func_name, edges);
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
