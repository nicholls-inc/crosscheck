use std::collections::{HashMap, HashSet};

use ruff_python_ast::{self as ast, Expr, Stmt};

use crate::db::{ContractRole, ConstraintType};

/// A contract inferred from function body analysis.
#[derive(Debug)]
pub struct BodyContract {
    pub constraint_type: ConstraintType,
    pub role: ContractRole,
    pub param_value: Option<i64>,
    pub param_nullable: Option<i64>,
}

/// Analyze a function body to infer contracts.
pub fn analyze_body(body: &[Stmt]) -> Vec<BodyContract> {
    let mut contracts = Vec::new();

    // Collect all return statements
    let returns = collect_returns(body);

    if returns.is_empty() {
        return contracts;
    }

    // Check for None returns (nullability)
    let has_none_return = returns.iter().any(is_none_return);
    if has_none_return {
        contracts.push(BodyContract {
            constraint_type: ConstraintType::Nullability,
            role: ContractRole::Postcondition,
            param_value: None,
            param_nullable: Some(1),
        });
    }

    // Precision of local variables, so `x = v.quantize(...); return f(field=x)` is seen.
    let env = local_precisions(body);

    // Check for precision patterns across all non-None return expressions.
    // A None return has no decimal places; it is covered by the nullability
    // contract above.
    let precisions: Vec<Option<i64>> = returns
        .iter()
        .filter(|r| !is_none_return(r))
        .filter_map(|r| r.as_ref())
        .map(|expr| analyze_precision(expr, &env))
        .collect();

    // Take the weakest (largest) precision bound
    if !precisions.is_empty() && precisions.iter().all(|p| p.is_some()) {
        let max_precision = precisions.iter().filter_map(|p| *p).max().unwrap_or(0);
        contracts.push(BodyContract {
            constraint_type: ConstraintType::Precision,
            role: ContractRole::Postcondition,
            param_value: Some(max_precision),
            param_nullable: None,
        });
    }

    contracts
}

/// Collect all return expressions from a function body.
fn collect_returns(stmts: &[Stmt]) -> Vec<Option<&Expr>> {
    let mut returns = Vec::new();
    for stmt in stmts {
        collect_returns_from_stmt(stmt, &mut returns);
    }
    returns
}

/// Recursively collect return expressions from a statement.
fn collect_returns_from_stmt<'a>(stmt: &'a Stmt, returns: &mut Vec<Option<&'a Expr>>) {
    match stmt {
        Stmt::Return(ret) => {
            returns.push(ret.value.as_deref());
        }
        Stmt::If(if_stmt) => {
            for s in &if_stmt.body {
                collect_returns_from_stmt(s, returns);
            }
            for s in &if_stmt.elif_else_clauses {
                for stmt in &s.body {
                    collect_returns_from_stmt(stmt, returns);
                }
            }
        }
        Stmt::For(for_stmt) => {
            for s in &for_stmt.body {
                collect_returns_from_stmt(s, returns);
            }
        }
        Stmt::While(while_stmt) => {
            for s in &while_stmt.body {
                collect_returns_from_stmt(s, returns);
            }
        }
        Stmt::Try(try_stmt) => {
            for s in &try_stmt.body {
                collect_returns_from_stmt(s, returns);
            }
            for handler in &try_stmt.handlers {
                let ast::ExceptHandler::ExceptHandler(h) = handler;
                for s in &h.body {
                    collect_returns_from_stmt(s, returns);
                }
            }
            for s in &try_stmt.orelse {
                collect_returns_from_stmt(s, returns);
            }
            for s in &try_stmt.finalbody {
                collect_returns_from_stmt(s, returns);
            }
        }
        Stmt::With(with_stmt) => {
            for s in &with_stmt.body {
                collect_returns_from_stmt(s, returns);
            }
        }
        _ => {}
    }
}

/// Known decimal places of local variables.
type Env = HashMap<String, i64>;

/// Upper bound on the decimal places of each local variable, taken over every
/// assignment to it in the body (flow-insensitive). A variable is left out if
/// any assignment to it has unknown precision, if it is rebound by a
/// for-loop, augmented assignment, tuple unpacking or `with ... as`, or if its
/// bound does not settle (e.g. `x = x * r` in a loop).
fn local_precisions(body: &[Stmt]) -> Env {
    let mut assignments: Vec<(String, &Expr)> = Vec::new();
    let mut opaque: HashSet<String> = HashSet::new();
    collect_assignments(body, &mut assignments, &mut opaque);

    let mut env = Env::new();
    const MAX_PASSES: usize = 4;
    for pass in 0..=MAX_PASSES {
        let mut next = Env::new();
        let mut unknown: HashSet<String> = opaque.clone();
        for (name, value) in &assignments {
            if unknown.contains(name) {
                continue;
            }
            match analyze_precision(value, &env) {
                Some(p) => {
                    let entry = next.entry(name.clone()).or_insert(p);
                    *entry = (*entry).max(p);
                }
                None => {
                    unknown.insert(name.clone());
                    next.remove(name);
                }
            }
        }
        if next == env {
            return env;
        }
        if pass == MAX_PASSES {
            // Still growing: drop the variables whose bound changed.
            next.retain(|k, v| env.get(k) == Some(v));
            return next;
        }
        env = next;
    }
    env
}

/// Collect `name = expr` assignments; names bound any other way are opaque.
fn collect_assignments<'a>(
    stmts: &'a [Stmt],
    out: &mut Vec<(String, &'a Expr)>,
    opaque: &mut HashSet<String>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Assign(assign) => {
                for target in &assign.targets {
                    match target {
                        Expr::Name(n) => out.push((n.id.to_string(), &assign.value)),
                        other => mark_opaque(other, opaque),
                    }
                }
            }
            Stmt::AnnAssign(ann) => {
                if let (Expr::Name(n), Some(value)) = (ann.target.as_ref(), &ann.value) {
                    out.push((n.id.to_string(), value));
                }
            }
            Stmt::AugAssign(aug) => mark_opaque(&aug.target, opaque),
            Stmt::If(if_stmt) => {
                collect_assignments(&if_stmt.body, out, opaque);
                for clause in &if_stmt.elif_else_clauses {
                    collect_assignments(&clause.body, out, opaque);
                }
            }
            Stmt::For(for_stmt) => {
                mark_opaque(&for_stmt.target, opaque);
                collect_assignments(&for_stmt.body, out, opaque);
                collect_assignments(&for_stmt.orelse, out, opaque);
            }
            Stmt::While(while_stmt) => {
                collect_assignments(&while_stmt.body, out, opaque);
                collect_assignments(&while_stmt.orelse, out, opaque);
            }
            Stmt::With(with_stmt) => {
                for item in &with_stmt.items {
                    if let Some(vars) = &item.optional_vars {
                        mark_opaque(vars, opaque);
                    }
                }
                collect_assignments(&with_stmt.body, out, opaque);
            }
            Stmt::Try(try_stmt) => {
                collect_assignments(&try_stmt.body, out, opaque);
                for handler in &try_stmt.handlers {
                    let ast::ExceptHandler::ExceptHandler(h) = handler;
                    if let Some(name) = &h.name {
                        opaque.insert(name.to_string());
                    }
                    collect_assignments(&h.body, out, opaque);
                }
                collect_assignments(&try_stmt.orelse, out, opaque);
                collect_assignments(&try_stmt.finalbody, out, opaque);
            }
            _ => {}
        }
    }
}

fn mark_opaque(target: &Expr, opaque: &mut HashSet<String>) {
    match target {
        Expr::Name(n) => {
            opaque.insert(n.id.to_string());
        }
        Expr::Tuple(t) => t.elts.iter().for_each(|e| mark_opaque(e, opaque)),
        Expr::List(l) => l.elts.iter().for_each(|e| mark_opaque(e, opaque)),
        Expr::Starred(st) => mark_opaque(&st.value, opaque),
        _ => {}
    }
}

/// Check if a return expression is None.
fn is_none_return(expr: &Option<&Expr>) -> bool {
    match expr {
        Some(Expr::NoneLiteral(_)) => true,
        None => true, // bare `return`
        _ => false,
    }
}

/// Analyze an expression for precision patterns.
/// Returns the number of decimal places if a precision pattern is detected.
fn analyze_precision(expr: &Expr, env: &Env) -> Option<i64> {
    match expr {
        // Tuple return: analyze each element and take the max
        Expr::Tuple(tuple) => {
            let precisions: Vec<Option<i64>> =
                tuple.elts.iter().map(|e| analyze_precision(e, env)).collect();
            if precisions.iter().all(|p| p.is_some()) {
                precisions.into_iter().flatten().max()
            } else {
                None
            }
        }
        // Method call: check for .quantize() or round()
        Expr::Call(call) => analyze_call_precision(call, env),
        // Binary operation: precision widening
        Expr::BinOp(binop) => analyze_binop_precision(binop, env),
        // Name reference: a local variable with known precision
        Expr::Name(name) => env.get(name.id.as_str()).copied(),
        _ => None,
    }
}

/// Analyze a call expression for precision patterns.
fn analyze_call_precision(call: &ast::ExprCall, env: &Env) -> Option<i64> {
    match call.func.as_ref() {
        // value.quantize(Decimal('0.001')) or value.quantize(step) with a known step
        Expr::Attribute(attr) if attr.attr.as_str() == "quantize" => {
            if let Some(arg) = call.arguments.args.first() {
                extract_decimal_precision(arg).or_else(|| analyze_precision(arg, env))
            } else {
                None
            }
        }
        // Decimal('0.25') literal: its own number of decimal places
        Expr::Name(name) if name.id.as_str() == "Decimal" && call.arguments.keywords.is_empty() => {
            match call.arguments.args.first() {
                Some(Expr::StringLiteral(_)) => extract_decimal_precision(&Expr::Call(call.clone())),
                Some(Expr::NumberLiteral(n)) if matches!(n.value, ast::Number::Int(_)) => Some(0),
                _ => None,
            }
        }
        // round(value, n)
        Expr::Name(name) if name.id.as_str() == "round" => {
            if call.arguments.args.len() >= 2 {
                if let Expr::NumberLiteral(num) = &call.arguments.args[1] {
                    match &num.value {
                        ast::Number::Int(i) => i.as_i64(),
                        _ => None,
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }
        // For other calls (e.g. Model.objects.create(field=val.quantize(...))),
        // analyze keyword argument values for precision patterns.
        _ => {
            let kwarg_precisions: Vec<i64> = call
                .arguments
                .keywords
                .iter()
                .filter_map(|kw| {
                    kw.arg.as_ref()?; // skip **kwargs
                    analyze_precision(&kw.value, env)
                })
                .collect();
            if kwarg_precisions.is_empty() {
                None
            } else {
                kwarg_precisions.into_iter().max()
            }
        }
    }
}

/// Extract precision from a Decimal('0.001') style argument.
fn extract_decimal_precision(expr: &Expr) -> Option<i64> {
    match expr {
        Expr::Call(call) => {
            // Decimal('0.001')
            if let Some(arg) = call.arguments.args.first() {
                if let Expr::StringLiteral(s) = arg {
                    let val = s.value.to_string();
                    if let Some(dot_pos) = val.find('.') {
                        let decimal_part = &val[dot_pos + 1..];
                        // Count significant digits
                        Some(
                            decimal_part
                                .trim_end_matches('0')
                                .len()
                                .max(decimal_part.len())
                                as i64,
                        )
                    } else {
                        Some(0)
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Analyze binary operation for precision widening.
fn analyze_binop_precision(binop: &ast::ExprBinOp, env: &Env) -> Option<i64> {
    let left_prec = analyze_precision(&binop.left, env);
    let right_prec = analyze_precision(&binop.right, env);

    match (left_prec, right_prec) {
        (Some(l), Some(r)) => match binop.op {
            ast::Operator::Add | ast::Operator::Sub => Some(l.max(r) + 1),
            ast::Operator::Mult => Some(l + r),
            ast::Operator::Div => Some(l + r), // conservative estimate
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: parse a Python expression and return the first Expr from the module.
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

    /// Helper: parse a function and run analyze_body on it.
    fn analyze_function(source: &str) -> Vec<BodyContract> {
        let parsed = ruff_python_parser::parse_unchecked(
            source,
            ruff_python_parser::Mode::Module.into(),
        );
        match parsed.into_syntax() {
            ruff_python_ast::Mod::Module(module) => {
                if let Stmt::FunctionDef(func) = &module.body[0] {
                    analyze_body(&func.body)
                } else {
                    panic!("expected function def");
                }
            }
            _ => panic!("expected module"),
        }
    }

    // -- extract_decimal_precision tests --

    #[test]
    fn test_decimal_precision_standard() {
        // Decimal('0.001') → 3 decimal places
        let expr = parse_expr("Decimal('0.001')");
        assert_eq!(extract_decimal_precision(&expr), Some(3));
    }

    #[test]
    fn test_decimal_precision_six_places() {
        let expr = parse_expr("Decimal('0.000001')");
        assert_eq!(extract_decimal_precision(&expr), Some(6));
    }

    #[test]
    fn test_decimal_precision_one_place() {
        let expr = parse_expr("Decimal('0.1')");
        assert_eq!(extract_decimal_precision(&expr), Some(1));
    }

    #[test]
    fn test_decimal_precision_integer() {
        let expr = parse_expr("Decimal('1')");
        assert_eq!(extract_decimal_precision(&expr), Some(0));
    }

    /// Documents the known dead code issue: trim_end_matches('0') is neutralized by .max().
    /// '0.0010' has 4 chars after the dot, and the function returns 4 (not 3).
    /// In Python, Decimal('0.001') and Decimal('0.0010') quantize identically to 3dp,
    /// so reporting 4 is technically over-counting. This test documents current behavior.
    #[test]
    fn test_decimal_precision_trailing_zeros_current_behavior() {
        let expr = parse_expr("Decimal('0.0010')");
        // Current behavior: returns 4 (counts all digits including trailing zero)
        // Correct behavior would be 3 (trailing zeros don't affect quantize precision)
        assert_eq!(extract_decimal_precision(&expr), Some(4));
    }

    // -- is_none_return tests --

    #[test]
    fn test_is_none_return_none_literal() {
        let expr = parse_expr("None");
        assert!(is_none_return(&Some(&expr)));
    }

    #[test]
    fn test_is_none_return_bare() {
        // bare return has no expression
        assert!(is_none_return(&None));
    }

    #[test]
    fn test_is_none_return_value() {
        let expr = parse_expr("42");
        assert!(!is_none_return(&Some(&expr)));
    }

    // -- collect_returns in nested control flow --

    #[test]
    fn test_returns_in_try_except() {
        let contracts = analyze_function(
            "def f(x):\n    try:\n        return round(x, 3)\n    except:\n        return round(x, 5)",
        );
        let precision = contracts
            .iter()
            .find(|c| c.constraint_type.as_str() == "precision");
        assert!(precision.is_some(), "should find precision in try/except");
        // Weakest branch: max(3, 5) = 5
        assert_eq!(precision.unwrap().param_value, Some(5));
    }

    #[test]
    fn test_returns_in_for_loop() {
        let contracts = analyze_function(
            "def f(items):\n    for x in items:\n        return round(x, 2)\n    return round(0, 4)",
        );
        let precision = contracts
            .iter()
            .find(|c| c.constraint_type.as_str() == "precision");
        assert!(precision.is_some());
        assert_eq!(precision.unwrap().param_value, Some(4));
    }

    #[test]
    fn test_returns_in_while() {
        let contracts =
            analyze_function("def f(x):\n    while x > 0:\n        return round(x, 7)");
        let precision = contracts
            .iter()
            .find(|c| c.constraint_type.as_str() == "precision");
        assert_eq!(precision.unwrap().param_value, Some(7));
    }

    fn precision_of(contracts: &[BodyContract]) -> Option<i64> {
        contracts
            .iter()
            .find(|c| c.constraint_type.as_str() == "precision")
            .and_then(|c| c.param_value)
    }

    #[test]
    fn test_precision_traced_through_local_variable() {
        let cs = analyze_function(
            "def f(a):\n    x = a.quantize(Decimal('0.0001'))\n    return Invoice(total=x)\n",
        );
        assert_eq!(precision_of(&cs), Some(4));
    }

    #[test]
    fn test_none_return_does_not_hide_precision() {
        let cs = analyze_function(
            "def f(a):\n    if a is None:\n        return None\n    return a.quantize(Decimal('0.01'))\n",
        );
        assert_eq!(precision_of(&cs), Some(2));
        assert!(cs.iter().any(|c| c.constraint_type.as_str() == "nullability"));
    }

    #[test]
    fn test_quantize_with_named_step() {
        let cs = analyze_function(
            "def f(a):\n    step = Decimal('0.001')\n    return a.quantize(step)\n",
        );
        assert_eq!(precision_of(&cs), Some(3));
    }

    #[test]
    fn test_reassignment_takes_max_and_unknown_poisons() {
        let cs = analyze_function(
            "def f(a, b):\n    x = a.quantize(Decimal('0.1'))\n    if b:\n        x = a.quantize(Decimal('0.001'))\n    return x\n",
        );
        assert_eq!(precision_of(&cs), Some(3));
        let cs = analyze_function(
            "def f(a, b):\n    x = a.quantize(Decimal('0.1'))\n    if b:\n        x = a\n    return x\n",
        );
        assert_eq!(precision_of(&cs), None);
    }

    #[test]
    fn test_loop_growth_is_not_bounded() {
        let cs = analyze_function(
            "def f(a, n):\n    x = Decimal('0.1')\n    while n:\n        x = x * Decimal('0.1')\n    return x\n",
        );
        assert_eq!(precision_of(&cs), None);
        let cs = analyze_function(
            "def f(xs):\n    x = Decimal('0.1')\n    for x in xs:\n        pass\n    return x\n",
        );
        assert_eq!(precision_of(&cs), None);
    }
}
