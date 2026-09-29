//! Property-based tests for docstring_parser.rs.
//!
//! Tests parsing of requires:/ensures: clauses in docstrings:
//! - Integer precision values round-trip exactly
//! - Dependent expressions with input_ are preserved verbatim
//! - requires → Precondition, ensures → Postcondition
//! - Non-clause lines produce nothing

use crosscheck_contracts::docstring_parser::parse_docstring;
use proptest::prelude::*;

proptest! {
    /// Integer precision values parse correctly for ensures clauses.
    #[test]
    fn prop_ensures_precision_roundtrip(n in 0..1000i64) {
        let docstring = format!("ensures: precision(result) <= {n}");
        let contracts = parse_docstring(&docstring);
        prop_assert_eq!(contracts.len(), 1, "expected exactly 1 contract");
        prop_assert_eq!(contracts[0].constraint_type.as_str(), "precision");
        prop_assert_eq!(contracts[0].role.as_str(), "postcondition");
        prop_assert_eq!(contracts[0].param_value, Some(n));
        prop_assert!(contracts[0].dependent_expr.is_none());
    }

    /// Integer precision values parse correctly for requires clauses.
    #[test]
    fn prop_requires_precision_roundtrip(n in 0..1000i64) {
        let docstring = format!("requires: precision(result) <= {n}");
        let contracts = parse_docstring(&docstring);
        prop_assert_eq!(contracts.len(), 1, "expected exactly 1 contract");
        prop_assert_eq!(contracts[0].constraint_type.as_str(), "precision");
        prop_assert_eq!(contracts[0].role.as_str(), "precondition");
        prop_assert_eq!(contracts[0].param_value, Some(n));
    }

    /// ensures always maps to postcondition, requires to precondition.
    #[test]
    fn prop_role_mapping(n in 0..100i64) {
        let ensures = parse_docstring(&format!("ensures: precision(result) <= {n}"));
        let requires = parse_docstring(&format!("requires: precision(result) <= {n}"));

        prop_assert_eq!(ensures[0].role.as_str(), "postcondition",
            "ensures should map to postcondition");
        prop_assert_eq!(requires[0].role.as_str(), "precondition",
            "requires should map to precondition");
    }

    /// Non-clause lines produce empty results.
    /// Any line not starting with "requires:" or "ensures:" should be ignored.
    #[test]
    fn prop_non_clause_ignored(s in "[^re\n][^\n]{0,80}") {
        let contracts = parse_docstring(&s);
        prop_assert!(contracts.is_empty(),
            "non-clause line should produce no contracts, but got {} from {:?}",
            contracts.len(), s);
    }

    /// Range constraints parse correctly.
    #[test]
    fn prop_range_min(n in 0..1000i64) {
        let docstring = format!("requires: result >= {n}");
        let contracts = parse_docstring(&docstring);
        prop_assert_eq!(contracts.len(), 1);
        prop_assert_eq!(contracts[0].constraint_type.as_str(), "range");
        prop_assert_eq!(contracts[0].param_value, Some(n));
    }
}

// -- Dependent expression tests --

#[test]
fn test_dependent_expr_max() {
    let contracts = parse_docstring("ensures: precision(result) <= max(input_precision, 3)");
    assert_eq!(contracts.len(), 1);
    assert_eq!(contracts[0].constraint_type.as_str(), "precision");
    assert_eq!(
        contracts[0].dependent_expr.as_deref(),
        Some("max(input_precision, 3)")
    );
    assert!(contracts[0].param_value.is_none());
}

#[test]
fn test_dependent_expr_min() {
    let contracts = parse_docstring("ensures: precision(result) <= min(input_a, input_b)");
    assert_eq!(contracts.len(), 1);
    assert_eq!(
        contracts[0].dependent_expr.as_deref(),
        Some("min(input_a, input_b)")
    );
}

#[test]
fn test_dependent_expr_input_alone() {
    let contracts = parse_docstring("ensures: precision(result) <= input_precision");
    assert_eq!(contracts.len(), 1);
    assert_eq!(
        contracts[0].dependent_expr.as_deref(),
        Some("input_precision")
    );
}

#[test]
fn test_nullable_result() {
    let contracts = parse_docstring("ensures: nullable(result)");
    assert_eq!(contracts.len(), 1);
    assert_eq!(contracts[0].constraint_type.as_str(), "nullability");
    assert_eq!(contracts[0].role.as_str(), "postcondition");
}

#[test]
fn test_input_param_precision() {
    let contracts = parse_docstring("requires: precision(offpeak) <= 10");
    assert_eq!(contracts.len(), 1);
    assert_eq!(contracts[0].constraint_type.as_str(), "precision");
    assert_eq!(contracts[0].param_value, Some(10));
    assert_eq!(contracts[0].role.as_str(), "precondition");
}

#[test]
fn test_multiline_docstring() {
    let docstring = "Compute energy values.\n\nrequires: precision(x) <= 5\nensures: precision(result) <= 3\nensures: nullable(result)";
    let contracts = parse_docstring(docstring);
    assert_eq!(contracts.len(), 3, "should parse all 3 clauses");
}
