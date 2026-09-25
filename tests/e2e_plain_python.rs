//! E2E tests for the plain-Python fixture: no Django, data classes as targets.
//!
//! Fixture: test_fixtures/plain_python/
//!   - records.py: `LineItem` (@dataclass) and `InvoiceRecord` (pydantic BaseModel)
//!   - pricing.py: functions whose results are written to those fields directly,
//!     through a local variable, positionally, or via another function
//!
//! The checker's verdicts on this fixture are covered by scripts/check-fixtures.sh.

mod test_helpers;

use std::path::Path;
use test_helpers::*;
use tempfile::TempDir;

fn plain_db() -> (TempDir, rusqlite::Connection) {
    let tmp = TempDir::new().unwrap();
    let conn = extract_to_db(Path::new("test_fixtures/plain_python"), &tmp);
    (tmp, conn)
}

fn only(contracts: Vec<ContractRow>, role: &str) -> ContractRow {
    let mut matching: Vec<ContractRow> = contracts
        .into_iter()
        .filter(|c| c.contract_role.as_deref() == Some(role))
        .collect();
    assert_eq!(matching.len(), 1, "expected exactly one {role}, got {matching:?}");
    matching.remove(0)
}

fn has_edge(edges: &[EdgeRow], source: &str, target: &str, relationship: &str) -> bool {
    edges
        .iter()
        .any(|e| e.source_name == source && e.target_name == target && e.relationship == relationship)
}

#[test]
fn test_data_class_fields_are_model_nodes() {
    let (_tmp, conn) = plain_db();
    let mut names: Vec<String> = query_nodes(&conn, "model").into_iter().map(|n| n.name).collect();
    names.sort();
    assert_eq!(
        names,
        [
            "InvoiceRecord.customer",
            "InvoiceRecord.discount_pct",
            "InvoiceRecord.note",
            "InvoiceRecord.total",
            "LineItem.quantity",
            "LineItem.sku",
            "LineItem.unit_price",
        ]
    );
}

#[test]
fn test_pydantic_field_preconditions() {
    let (_tmp, conn) = plain_db();

    let length = only(query_contract_by_type(&conn, "InvoiceRecord.customer", "length"), "precondition");
    assert_eq!(length.param_max_length, Some(32));

    let precision = only(query_contract_by_type(&conn, "InvoiceRecord.total", "precision"), "precondition");
    assert_eq!((precision.param_max_digits, precision.param_decimal_places), (Some(10), Some(2)));

    let null = only(query_contract_by_type(&conn, "InvoiceRecord.note", "nullability"), "precondition");
    assert_eq!(null.param_nullable, Some(1), "Optional[str] accepts None");
    let null = only(query_contract_by_type(&conn, "InvoiceRecord.total", "nullability"), "precondition");
    assert_eq!(null.param_nullable, Some(0), "Decimal does not accept None");
    assert!(!null.is_implicit, "nullability comes from the annotation");

    let ty = only(query_contract_by_type(&conn, "LineItem.unit_price", "type"), "precondition");
    assert_eq!(ty.param_type_name.as_deref(), Some("Decimal"));
}

#[test]
fn test_function_postconditions() {
    let (_tmp, conn) = plain_db();

    let null = only(query_contract_by_type(&conn, "lookup_discount", "nullability"), "postcondition");
    assert_eq!(null.param_nullable, Some(1), "Optional[int] return");
    let ty = only(query_contract_by_type(&conn, "lookup_discount", "type"), "postcondition");
    assert_eq!(ty.param_type_name.as_deref(), Some("int"), "Optional[int] unwraps to int");

    let length = only(query_contract_by_type(&conn, "customer_label", "length"), "postcondition");
    assert_eq!((length.param_max_length, length.verification_level.as_str()), (Some(64), "ASSUMED"));

    let precision = only(query_contract_by_type(&conn, "with_tax", "precision"), "postcondition");
    assert_eq!((precision.param_decimal_places, precision.verification_level.as_str()), (Some(4), "EXTRACTED"));
}

#[test]
fn test_data_flow_edges() {
    let (_tmp, conn) = plain_db();
    let edges = query_edges(&conn);

    // Cls(field=g(...)): the producing function writes the field
    assert!(has_edge(&edges, "lookup_discount", "InvoiceRecord.discount_pct", "writes_to"));
    assert!(has_edge(&edges, "customer_label", "InvoiceRecord.customer", "writes_to"));
    // x = g(...); Cls(field=x)
    assert!(has_edge(&edges, "normalise", "InvoiceRecord.total", "writes_to"));
    // h(g(...))
    assert!(has_edge(&edges, "with_tax", "normalise", "flows_to"));
    // Positional dataclass construction: LineItem(sku, with_tax(price), quantity)
    assert!(has_edge(&edges, "with_tax", "LineItem.unit_price", "writes_to"));
    assert!(has_edge(&edges, "build_line_item", "LineItem.quantity", "writes_to"));
    // v2: a value produced by another function is written by that function
    // only; the enclosing function keeps its `calls` edges and passes its own
    // expressions on through `flows_to` edges with override rows.
    assert!(!has_edge(&edges, "record_invoice", "InvoiceRecord.total", "writes_to"));
    assert!(has_edge(&edges, "record_invoice", "normalise", "calls"));
    let e = the_edge(&edges, "with_tax", "normalise", "flows_to", Some("amount"));
    assert!(!e.source_override);
    assert_eq!(
        override_rows(&conn, &edges, "build_line_item", "LineItem.sku", "writes_to", None),
        ["nullability=0", "type=str"]
    );
    assert_eq!(
        override_rows(&conn, &edges, "record_invoice", "customer_label", "flows_to", Some("first")),
        ["nullability=0", "type=str"]
    );
    // Producer edges carry no override.
    for (src, tgt) in [
        ("lookup_discount", "InvoiceRecord.discount_pct"),
        ("customer_label", "InvoiceRecord.customer"),
        ("normalise", "InvoiceRecord.total"),
        ("with_tax", "LineItem.unit_price"),
    ] {
        assert!(!the_edge(&edges, src, tgt, "writes_to", None).source_override, "{src} -> {tgt}");
    }
    // Builtins and methods (DISCOUNT_CODES.get, str.upper) produce no edges
    assert!(edges.iter().all(|e| e.discovery == "ast_pattern"));
    assert_eq!(edges.len(), 17, "{edges:#?}");
}

#[test]
fn test_source_lines_are_line_numbers() {
    let (_tmp, conn) = plain_db();
    let line_of = |name: &str| {
        query_nodes(&conn, "function")
            .into_iter()
            .chain(query_nodes(&conn, "model"))
            .find(|n| n.name == name)
            .map(|n| n.source_line)
    };
    assert_eq!(line_of("lookup_discount"), Some(9));
    assert_eq!(line_of("InvoiceRecord.total"), Some(17));
}
