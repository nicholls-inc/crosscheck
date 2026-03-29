//! Property-based tests for body_analyzer.rs.
//!
//! Tests the precision arithmetic that determines function postconditions:
//! - quantize(Decimal('0.xxx')) → decimal place count
//! - round(val, n) → n decimal places
//! - binary ops: add/sub → max(l,r)+1, mult → l+r
//! - multiple returns → weakest (largest) bound

mod test_helpers;

use crosscheck_contracts::body_analyzer::analyze_body;
use proptest::prelude::*;
use test_helpers::parse_python_stmts;

/// Parse a function body and return the precision postcondition value (if any).
fn precision_from_source(source: &str) -> Option<i64> {
    let stmts = parse_python_stmts(source);
    // Find the function def and analyze its body
    for stmt in &stmts {
        if let ruff_python_ast::Stmt::FunctionDef(func) = stmt {
            let contracts = analyze_body(&func.body);
            for c in &contracts {
                if c.constraint_type.as_str() == "precision" {
                    return c.param_value;
                }
            }
        }
    }
    None
}

// -- Precision arithmetic properties --

proptest! {
    /// Addition/subtraction widens precision by 1: max(l, r) + 1
    #[test]
    fn prop_add_precision(l in 1..15i64, r in 1..15i64) {
        let source = format!(
            "def f(a, b):\n    return round(a, {}) + round(b, {})", l, r
        );
        let result = precision_from_source(&source);
        let expected = l.max(r) + 1;
        prop_assert_eq!(result, Some(expected));
    }

    #[test]
    fn prop_sub_precision(l in 1..15i64, r in 1..15i64) {
        let source = format!(
            "def f(a, b):\n    return round(a, {}) - round(b, {})", l, r
        );
        let result = precision_from_source(&source);
        let expected = l.max(r) + 1;
        prop_assert_eq!(result, Some(expected));
    }

    /// Multiplication sums precisions: l + r
    #[test]
    fn prop_mult_precision(l in 1..10i64, r in 1..10i64) {
        let source = format!(
            "def f(a, b):\n    return round(a, {}) * round(b, {})", l, r
        );
        let result = precision_from_source(&source);
        prop_assert_eq!(result, Some(l + r));
    }

    /// Commutativity: precision(l op r) == precision(r op l)
    #[test]
    fn prop_add_commutative(l in 1..15i64, r in 1..15i64) {
        let src_lr = format!("def f(a, b):\n    return round(a, {}) + round(b, {})", l, r);
        let src_rl = format!("def f(a, b):\n    return round(a, {}) + round(b, {})", r, l);
        prop_assert_eq!(precision_from_source(&src_lr), precision_from_source(&src_rl));
    }

    #[test]
    fn prop_mult_commutative(l in 1..10i64, r in 1..10i64) {
        let src_lr = format!("def f(a, b):\n    return round(a, {}) * round(b, {})", l, r);
        let src_rl = format!("def f(a, b):\n    return round(a, {}) * round(b, {})", r, l);
        prop_assert_eq!(precision_from_source(&src_lr), precision_from_source(&src_rl));
    }

    /// round(val, n) alone has precision n
    #[test]
    fn prop_round_precision(n in 0..20i64) {
        let source = format!("def f(x):\n    return round(x, {})", n);
        let result = precision_from_source(&source);
        prop_assert_eq!(result, Some(n));
    }

    /// Quantize with a Decimal string: count is the string's decimal digit count.
    /// Uses the pattern '0.' followed by (n-1) zeros then '1'.
    /// Note: current implementation counts ALL digits after the dot, including trailing zeros.
    #[test]
    fn prop_quantize_precision(n in 1..15u32) {
        let decimal_str = format!("0.{}1", "0".repeat((n - 1) as usize));
        let source = format!(
            "def f(x):\n    return x.quantize(Decimal('{}'))", decimal_str
        );
        let result = precision_from_source(&source);
        // The string has exactly n characters after the dot
        prop_assert_eq!(result, Some(n as i64));
    }

    /// Weakest branch wins: if two returns have different precisions, take the max.
    #[test]
    fn prop_weakest_branch(p1 in 1..15i64, p2 in 1..15i64) {
        let source = format!(
            "def f(x, cond):\n    if cond:\n        return round(x, {})\n    else:\n        return round(x, {})",
            p1, p2
        );
        let result = precision_from_source(&source);
        prop_assert_eq!(result, Some(p1.max(p2)));
    }
}

// -- Nullability property --

#[test]
fn test_none_return_detected() {
    let source = "def f(x):\n    if x:\n        return x\n    return None";
    let stmts = parse_python_stmts(source);
    if let ruff_python_ast::Stmt::FunctionDef(func) = &stmts[0] {
        let contracts = analyze_body(&func.body);
        let nullable = contracts
            .iter()
            .any(|c| c.constraint_type.as_str() == "nullability" && c.param_nullable == Some(1));
        assert!(nullable, "should detect None return as nullable postcondition");
    } else {
        panic!("expected function def");
    }
}

#[test]
fn test_bare_return_detected() {
    let source = "def f(x):\n    if x:\n        return x\n    return";
    let stmts = parse_python_stmts(source);
    if let ruff_python_ast::Stmt::FunctionDef(func) = &stmts[0] {
        let contracts = analyze_body(&func.body);
        let nullable = contracts
            .iter()
            .any(|c| c.constraint_type.as_str() == "nullability");
        assert!(nullable, "bare return should be detected as nullable");
    } else {
        panic!("expected function def");
    }
}
