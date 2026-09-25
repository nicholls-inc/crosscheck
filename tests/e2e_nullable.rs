//! E2E tests for the Nullable fixture: precision + nullability on one edge.
//!
//! Fixture: test_fixtures/nullable/
//!   - models.py: Invoice with DecimalField(max_digits=10, decimal_places=2), null=False by default
//!   - utils.py:  apply_discount() returns None on one path, otherwise writes 4dp values via ORM
//!
//! The Lean side of this fixture is covered by prover/ContractGraphTest/NullableDemo.lean.

mod test_helpers;

use std::path::Path;
use test_helpers::*;
use tempfile::TempDir;

fn nullable_db() -> (TempDir, rusqlite::Connection) {
    let tmp = TempDir::new().unwrap();
    let conn = extract_to_db(Path::new("test_fixtures/nullable"), &tmp);
    (tmp, conn)
}

fn postconditions<'a>(contracts: &'a [ContractRow]) -> Vec<&'a ContractRow> {
    contracts
        .iter()
        .filter(|c| c.contract_role.as_deref() == Some("postcondition"))
        .collect()
}

#[test]
fn test_nullable_model_preconditions() {
    let (_tmp, conn) = nullable_db();

    for field in &["Invoice.total", "Invoice.discount"] {
        let precision = query_contract_by_type(&conn, field, "precision");
        assert_eq!(precision.len(), 1, "{field}: expected 1 precision contract");
        assert_eq!(precision[0].param_decimal_places, Some(2));
        assert_eq!(precision[0].contract_role.as_deref(), Some("precondition"));

        let null = query_contract_by_type(&conn, field, "nullability");
        assert_eq!(null.len(), 1, "{field}: expected 1 nullability contract");
        assert_eq!(null[0].param_nullable, Some(0), "{field}: null=False by default");
        assert!(null[0].is_implicit, "{field}: null=False is the implicit default");
        assert_eq!(null[0].contract_role.as_deref(), Some("precondition"));
    }
}

#[test]
fn test_nullable_function_precision_from_docstring() {
    let (_tmp, conn) = nullable_db();

    let precision = query_contract_by_type(&conn, "apply_discount", "precision");
    let post = postconditions(&precision);
    assert_eq!(post.len(), 1, "expected exactly 1 precision postcondition");
    assert_eq!(post[0].param_decimal_places, Some(4));
    assert_eq!(post[0].verification_level, "ASSUMED");
}

#[test]
fn test_nullable_function_nullability_from_body() {
    let (_tmp, conn) = nullable_db();

    let null = query_contract_by_type(&conn, "apply_discount", "nullability");
    let post = postconditions(&null);
    assert_eq!(post.len(), 1, "expected exactly 1 nullability postcondition");
    assert_eq!(
        post[0].param_nullable,
        Some(1),
        "body analyzer should mark result nullable from `return None`"
    );
    assert_eq!(post[0].verification_level, "EXTRACTED");
}

#[test]
fn test_nullable_writes_to_edges() {
    let (_tmp, conn) = nullable_db();

    let edges = query_edges(&conn);
    let writes_to: Vec<&EdgeRow> = edges
        .iter()
        .filter(|e| e.relationship == "writes_to")
        .collect();
    assert_eq!(writes_to.len(), 2, "expected 2 writes_to edges");

    let targets: Vec<&str> = writes_to.iter().map(|e| e.target_name.as_str()).collect();
    assert!(targets.contains(&"Invoice.total"));
    assert!(targets.contains(&"Invoice.discount"));
    for edge in &writes_to {
        assert_eq!(edge.source_name, "apply_discount");
        assert_eq!(edge.discovery, "ast_pattern");
    }
}
