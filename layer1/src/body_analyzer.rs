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

    // Check for precision patterns across all return expressions
    let precisions: Vec<Option<i64>> = returns
        .iter()
        .filter_map(|r| r.as_ref())
        .map(|expr| analyze_precision(expr))
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
fn analyze_precision(expr: &Expr) -> Option<i64> {
    match expr {
        // Tuple return: analyze each element and take the max
        Expr::Tuple(tuple) => {
            let precisions: Vec<Option<i64>> =
                tuple.elts.iter().map(analyze_precision).collect();
            if precisions.iter().all(|p| p.is_some()) {
                precisions.into_iter().flatten().max()
            } else {
                None
            }
        }
        // Method call: check for .quantize() or round()
        Expr::Call(call) => analyze_call_precision(call),
        // Binary operation: precision widening
        Expr::BinOp(binop) => analyze_binop_precision(binop),
        // Name reference: no precision info
        _ => None,
    }
}

/// Analyze a call expression for precision patterns.
fn analyze_call_precision(call: &ast::ExprCall) -> Option<i64> {
    match call.func.as_ref() {
        // value.quantize(Decimal('0.001'))
        Expr::Attribute(attr) if attr.attr.as_str() == "quantize" => {
            if let Some(arg) = call.arguments.args.first() {
                extract_decimal_precision(arg)
            } else {
                None
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
                    analyze_precision(&kw.value)
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
fn analyze_binop_precision(binop: &ast::ExprBinOp) -> Option<i64> {
    let left_prec = analyze_precision(&binop.left);
    let right_prec = analyze_precision(&binop.right);

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
