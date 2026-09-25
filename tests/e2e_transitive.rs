//! E2E tests for the Transitive fixture: graph-level inconsistency.
//!
//! Fixture: test_fixtures/transitive/
//!   - models.py: EnergyRecord with DecimalField(max_digits=5, decimal_places=3)
//!   - utils.py:  compute_offpeak() -> split_energy() -> EnergyRecord
//!     - compute_offpeak: ensures precision(result) <= 4
//!     - split_energy: ensures precision(result) <= max(input_precision, 3)
//!
//! Covers user stories: 1.4 (function signatures, docstrings), 1.5 (edges, schema).

mod test_helpers;

use std::path::Path;
use test_helpers::*;
use tempfile::TempDir;

fn transitive_db() -> (TempDir, rusqlite::Connection) {
    let tmp = TempDir::new().unwrap();
    let conn = extract_to_db(Path::new("test_fixtures/transitive"), &tmp);
    (tmp, conn)
}

#[test]
fn test_transitive_docstring_contracts() {
    let (_tmp, conn) = transitive_db();

    // compute_offpeak should have a precision postcondition of 4, from docstring
    let co_precision = query_contract_by_type(&conn, "compute_offpeak", "precision");
    let co_post: Vec<&ContractRow> = co_precision
        .iter()
        .filter(|c| c.contract_role.as_deref() == Some("postcondition"))
        .collect();
    assert!(
        !co_post.is_empty(),
        "compute_offpeak should have a precision postcondition"
    );
    assert_eq!(
        co_post[0].param_decimal_places,
        Some(4),
        "compute_offpeak postcondition should be precision <= 4"
    );
    assert_eq!(
        co_post[0].verification_level, "ASSUMED",
        "docstring contracts should have verification level ASSUMED"
    );

    // split_energy should have a precision precondition of 10
    let se_precision = query_contract_by_type(&conn, "split_energy", "precision");
    let se_pre: Vec<&ContractRow> = se_precision
        .iter()
        .filter(|c| c.contract_role.as_deref() == Some("precondition"))
        .collect();
    assert!(
        !se_pre.is_empty(),
        "split_energy should have a precision precondition"
    );
    assert_eq!(
        se_pre[0].param_decimal_places,
        Some(10),
        "split_energy precondition should be precision(offpeak) <= 10"
    );

    // split_energy should have a dependent-expression postcondition
    let se_post: Vec<&ContractRow> = se_precision
        .iter()
        .filter(|c| c.contract_role.as_deref() == Some("postcondition"))
        .collect();
    assert!(
        !se_post.is_empty(),
        "split_energy should have a precision postcondition"
    );
    assert_eq!(
        se_post[0].dependent_expr.as_deref(),
        Some("max(input_precision, 3)"),
        "split_energy postcondition should have dependent expression"
    );
}

#[test]
fn test_transitive_calls_edge() {
    let (_tmp, conn) = transitive_db();

    let edges = query_edges(&conn);
    let calls: Vec<&EdgeRow> = edges
        .iter()
        .filter(|e| e.relationship == "calls")
        .collect();

    let has_offpeak_to_split = calls
        .iter()
        .any(|e| e.source_name == "compute_offpeak" && e.target_name == "split_energy");
    assert!(
        has_offpeak_to_split,
        "should have calls edge: compute_offpeak -> split_energy, edges: {calls:?}"
    );
}

#[test]
fn test_transitive_writes_to_edge() {
    let (_tmp, conn) = transitive_db();

    let edges = query_edges(&conn);
    let writes: Vec<&EdgeRow> = edges
        .iter()
        .filter(|e| e.relationship == "writes_to")
        .collect();

    let has_split_to_energy = writes
        .iter()
        .any(|e| e.source_name == "split_energy" && e.target_name == "EnergyRecord.energy");
    assert!(
        has_split_to_energy,
        "should have writes_to edge: split_energy -> EnergyRecord.energy, edges: {writes:?}"
    );
}

#[test]
fn test_transitive_docstring_dominates_body() {
    let (_tmp, conn) = transitive_db();

    // split_energy's body contains quantize() calls that would produce a concrete
    // precision postcondition. But the docstring provides a dependent-expression
    // postcondition. Docstring should dominate (only one precision postcondition).
    let precision = query_contract_by_type(&conn, "split_energy", "precision");
    let postconditions: Vec<&ContractRow> = precision
        .iter()
        .filter(|c| c.contract_role.as_deref() == Some("postcondition"))
        .collect();

    assert_eq!(
        postconditions.len(),
        1,
        "split_energy should have exactly 1 precision postcondition (docstring dominates body), got: {postconditions:?}"
    );
    assert!(
        postconditions[0].dependent_expr.is_some(),
        "the single precision postcondition should be the dependent-expression one from the docstring"
    );
}

/// v2 rows: the argument of `split_energy(offpeak)` is an expression in
/// compute_offpeak (override, bound to parameter `offpeak`), and the value
/// written by split_energy depends on its only input.
#[test]
fn test_transitive_v2_rows() {
    let (_tmp, conn) = transitive_db();
    let edges = query_edges(&conn);
    assert_eq!(edges.len(), 3, "{edges:#?}");
    the_edge(&edges, "compute_offpeak", "split_energy", "calls", None);
    assert_eq!(
        override_rows(&conn, &edges, "compute_offpeak", "split_energy", "flows_to", Some("offpeak")),
        ["nullability=0", "precision=4", "type=Decimal"]
    );
    assert_eq!(
        override_rows(&conn, &edges, "split_energy", "EnergyRecord.energy", "writes_to", None),
        ["nullability=0", "precision=max(3, input_precision)", "type=Decimal"]
    );
    assert_eq!(
        node_rows(&conn, "split_energy", "precondition"),
        [
            "offpeak: nullability=0",
            "offpeak: precision=10 [ASSUMED]",
            "offpeak: type=Decimal",
        ]
    );
}
