//! E2E tests for the Bug 1 fixture: precision mismatch.
//!
//! Fixture: test_fixtures/bug1/
//!   - models.py: EnergyRecord with DecimalField(max_digits=5, decimal_places=3)
//!   - utils.py:  split_energy() quantizes to 6dp then writes via ORM
//!
//! Covers user stories: 1.1 (explicit constraints), 1.2 (implicit defaults), 1.5 (SQLite schema).

mod test_helpers;

use std::path::Path;
use test_helpers::*;
use tempfile::TempDir;

fn bug1_db() -> (TempDir, rusqlite::Connection) {
    let tmp = TempDir::new().unwrap();
    let conn = extract_to_db(Path::new("test_fixtures/bug1"), &tmp);
    (tmp, conn)
}

#[test]
fn test_bug1_correct_nodes() {
    let (_tmp, conn) = bug1_db();

    let model_nodes = query_nodes(&conn, "model");
    let func_nodes = query_nodes(&conn, "function");

    let model_names: Vec<&str> = model_nodes.iter().map(|n| n.name.as_str()).collect();
    assert!(
        model_names.contains(&"EnergyRecord.energy"),
        "missing EnergyRecord.energy node, got: {model_names:?}"
    );
    assert!(
        model_names.contains(&"EnergyRecord.off_peak_energy"),
        "missing EnergyRecord.off_peak_energy node, got: {model_names:?}"
    );

    let func_names: Vec<&str> = func_nodes.iter().map(|n| n.name.as_str()).collect();
    assert!(
        func_names.contains(&"split_energy"),
        "missing split_energy node, got: {func_names:?}"
    );
}

#[test]
fn test_bug1_model_precision_3dp() {
    let (_tmp, conn) = bug1_db();

    let precision = query_contract_by_type(&conn, "EnergyRecord.energy", "precision");
    assert_eq!(precision.len(), 1, "expected exactly 1 precision contract");

    let c = &precision[0];
    assert_eq!(c.param_decimal_places, Some(3), "decimal_places should be 3");
    assert_eq!(c.param_max_digits, Some(5), "max_digits should be 5");
    assert_eq!(
        c.contract_role.as_deref(),
        Some("precondition"),
        "model field precision should be a precondition"
    );
    assert_eq!(c.verification_level, "EXTRACTED");
}

/// v2: the 6dp values are the arguments of `objects.create`, so they are the
/// override rows of the two `writes_to` edges; `split_energy`'s own
/// postconditions describe its return value (an EnergyRecord) and carry no
/// precision.
#[test]
fn test_bug1_function_precision_6dp() {
    let (_tmp, conn) = bug1_db();

    let precision = query_contract_by_type(&conn, "split_energy", "precision");
    assert!(
        precision.iter().all(|c| c.contract_role.as_deref() != Some("postcondition")),
        "split_energy's return value has no precision: {precision:?}"
    );

    let edges = query_edges(&conn);
    for field in ["EnergyRecord.energy", "EnergyRecord.off_peak_energy"] {
        assert_eq!(
            override_rows(&conn, &edges, "split_energy", field, "writes_to", None),
            ["nullability=0", "precision=6", "type=Decimal"],
            "body analyzer should infer 6dp from quantize(Decimal('0.000001')) for {field}"
        );
    }
}

#[test]
fn test_bug1_writes_to_edges() {
    let (_tmp, conn) = bug1_db();

    let edges = query_edges(&conn);
    // The `return` of split_energy (annotated `-> EnergyRecord`) is a write
    // to its return contract node (round 6).
    let returns: Vec<&EdgeRow> = edges
        .iter()
        .filter(|e| e.relationship == "writes_to" && e.target_name.ends_with(".<return>"))
        .collect();
    assert_eq!(returns.len(), 1, "one return site");
    assert_eq!(returns[0].target_name, "split_energy.<return>");
    let writes_to: Vec<&EdgeRow> = edges
        .iter()
        .filter(|e| e.relationship == "writes_to" && !e.target_name.ends_with(".<return>"))
        .collect();

    assert_eq!(writes_to.len(), 2, "expected 2 writes_to edges");

    let targets: Vec<&str> = writes_to.iter().map(|e| e.target_name.as_str()).collect();
    assert!(targets.contains(&"EnergyRecord.energy"));
    assert!(targets.contains(&"EnergyRecord.off_peak_energy"));

    for edge in &writes_to {
        assert_eq!(edge.source_name, "split_energy");
        assert_eq!(edge.discovery, "ast_pattern");
        assert!(edge.source_override, "the written values are expressions in split_energy");
        assert_eq!(edge.target_param, None);
    }
}

#[test]
fn test_bug1_implicit_null_defaults() {
    let (_tmp, conn) = bug1_db();

    for field in &["EnergyRecord.energy", "EnergyRecord.off_peak_energy"] {
        let null_contracts = query_contract_by_type(&conn, field, "nullability");
        assert!(
            !null_contracts.is_empty(),
            "{field} should have a nullability contract"
        );

        let c = &null_contracts[0];
        assert_eq!(
            c.param_nullable,
            Some(0),
            "{field}: null should be false (0) by default"
        );
        assert!(
            c.is_implicit,
            "{field}: null=false should be marked implicit (not in source)"
        );
        assert_eq!(
            c.contract_role.as_deref(),
            Some("precondition"),
            "{field}: nullability should be a precondition"
        );
    }
}

#[test]
fn test_bug1_type_contracts() {
    let (_tmp, conn) = bug1_db();

    // A DecimalField accepts int, float and Decimal alike (Django converts),
    // so it has no type contract (round 3, F4).
    let model_type = query_contract_by_type(&conn, "EnergyRecord.energy", "type");
    assert!(
        model_type.is_empty(),
        "EnergyRecord.energy (numeric field) should have no type contract, got {model_type:?}"
    );

    // split_energy returns EnergyRecord (a model class name), which is intentionally
    // NOT emitted as a type postcondition — only value types (Decimal, int, str, etc.)
    // are meaningful for contract comparison against model fields.
    let func_type = query_contract_by_type(&conn, "split_energy", "type");
    let model_return: Vec<&ContractRow> = func_type
        .iter()
        .filter(|c| c.contract_role.as_deref() == Some("postcondition"))
        .collect();
    assert!(
        model_return.is_empty(),
        "split_energy returns EnergyRecord — no type postcondition should be emitted for model class names"
    );
}
