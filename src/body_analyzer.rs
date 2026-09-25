//! Return-value contracts of a function body, without project context.
//!
//! The extractor itself uses `value_analysis::Scope::return_facts` with the
//! whole project in view; `analyze_body` runs the same analysis on a bare
//! body (no parameters, imports or other functions known), which is what
//! the property tests exercise.

use std::collections::HashMap;

use ruff_python_ast::Stmt;
#[cfg(test)]
use ruff_python_ast::Expr;

use crate::db::{ConstraintType, ContractRole};
use crate::flow::FunctionFlow;
use crate::resolve::ProjectIndex;
#[cfg(test)]
use crate::value_analysis::decimal_literal_places;
use crate::value_analysis::{Scope, ValueFacts};

/// A contract inferred from function body analysis.
#[derive(Debug)]
pub struct BodyContract {
    pub constraint_type: ConstraintType,
    pub role: ContractRole,
    pub param_value: Option<i64>,
    pub param_nullable: Option<i64>,
}

/// Analyze a function body to infer postconditions of its return value:
/// nullability (a `None` return, a None producer, or falling off the end
/// of a value-returning body) and a static decimal-place bound.
pub fn analyze_body(body: &[Stmt]) -> Vec<BodyContract> {
    let index = ProjectIndex::default();
    let summaries: HashMap<String, ValueFacts> = HashMap::new();
    let flow = FunctionFlow::of(body);
    let facts = Scope::new(&index, &summaries, None, &flow).return_facts();

    let mut contracts = Vec::new();
    if let Some(nullable) = facts.nullable {
        contracts.push(BodyContract {
            constraint_type: ConstraintType::Nullability,
            role: ContractRole::Postcondition,
            param_value: None,
            param_nullable: Some(nullable as i64),
        });
    }
    if let (false, Some(p)) = (facts.always_none, facts.precision.and_then(|p| p.as_static())) {
        contracts.push(BodyContract {
            constraint_type: ConstraintType::Precision,
            role: ContractRole::Postcondition,
            param_value: Some(p),
            param_nullable: None,
        });
    }
    contracts
}

/// Check if a return expression is None.
#[cfg(test)]
fn is_none_return(expr: &Option<&Expr>) -> bool {
    matches!(expr, Some(Expr::NoneLiteral(_)) | None)
}

/// Decimal places of a `Decimal('0.001')` call expression.
#[cfg(test)]
fn extract_decimal_precision(expr: &Expr) -> Option<i64> {
    match expr {
        Expr::Call(call) => match call.arguments.args.first() {
            Some(Expr::StringLiteral(s)) => decimal_literal_places(s.value.to_str()),
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
            "def f(a):\n    x = a.quantize(Decimal('0.0001'))\n    return x\n",
        );
        assert_eq!(precision_of(&cs), Some(4));
    }

    /// Postconditions describe the return value only: an object built from
    /// a 4dp value has no precision of its own (the write edge carries it).
    #[test]
    fn test_constructed_object_has_no_precision() {
        let cs = analyze_function(
            "def f(a):\n    x = a.quantize(Decimal('0.0001'))\n    return Invoice(total=x)\n",
        );
        assert_eq!(precision_of(&cs), None);
    }

    #[test]
    fn test_fall_through_and_none_producers_are_nullable() {
        let nullable = |src: &str| {
            analyze_function(src)
                .iter()
                .find(|c| c.constraint_type.as_str() == "nullability")
                .and_then(|c| c.param_nullable)
        };
        assert_eq!(nullable("def f(x):\n    if x:\n        return 'a'\n"), Some(1));
        assert_eq!(nullable("def f(d, k):\n    return d.get(k)\n"), Some(1));
        assert_eq!(nullable("def f(x):\n    return 'a'\n"), Some(0));
        assert_eq!(nullable("def f(x):\n    return x\n"), None);
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
