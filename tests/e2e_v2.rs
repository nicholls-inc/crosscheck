//! E2E extraction tests for the `test_fixtures/v2_*` fixtures: the rows each
//! fixture's verdict depends on.

mod test_helpers;

use std::path::Path;
use tempfile::TempDir;
use test_helpers::*;

fn db(fixture: &str) -> (TempDir, rusqlite::Connection) {
    let tmp = TempDir::new().unwrap();
    let conn = extract_to_db(&Path::new("test_fixtures").join(fixture), &tmp);
    (tmp, conn)
}

fn producer(edges: &[EdgeRow], source: &str, target: &str, rel: &str, param: Option<&str>) {
    let e = the_edge(edges, source, target, rel, param);
    assert!(
        !e.source_override,
        "{source} -> {target} should use {source}'s postconditions"
    );
}

#[test]
fn test_v2_dependent_resolved() {
    let (_t, conn) = db("v2_dependent_resolved");
    let edges = query_edges(&conn);
    producer(&edges, "make4dp", "clamp_floor", "flows_to", Some("p"));
    producer(&edges, "clamp_floor", "Invoice.total", "writes_to", None);
    assert_eq!(
        node_rows(&conn, "clamp_floor", "postcondition"),
        [
            "nullability=0",
            "precision=max(3, input_precision)",
            "type=Decimal"
        ]
    );
}

#[test]
fn test_v2_dependent_unknown_multiparam() {
    let (_t, conn) = db("v2_dependent_unknown_multiparam");
    assert_eq!(
        node_rows(&conn, "clamp_floor", "postcondition"),
        ["nullability=0", "type=Decimal"]
    );
}

#[test]
fn test_v2_django_bounds() {
    let (_t, conn) = db("v2_django_bounds");
    assert_eq!(
        node_rows(&conn, "Meter.reading", "precondition"),
        ["nullability=0", "range=0..100"]
    );
    assert_eq!(
        node_rows(&conn, "Meter.count", "precondition"),
        ["nullability=0", "range=0.."]
    );
    let edges = query_edges(&conn);
    producer(&edges, "adjust_reading", "Meter.reading", "writes_to", None);
    producer(&edges, "bump_count", "Meter.count", "writes_to", None);
}

#[test]
fn test_v2_docstring_write_call() {
    let (_t, conn) = db("v2_docstring_write_call");
    let edges = query_edges(&conn);
    // `return Reading.objects.create(kwh=v)`: the docstring's static bound is
    // added next to the bound that depends on the only input.
    assert_eq!(
        override_rows(
            &conn,
            &edges,
            "record_reading",
            "Reading.kwh",
            "writes_to",
            None
        ),
        [
            "nullability=0",
            "precision=5 [ASSUMED]",
            "precision=input_precision",
            "type=Decimal"
        ]
    );
}

#[test]
fn test_v2_import_alias() {
    let (_t, conn) = db("v2_import_alias");
    producer(
        &query_edges(&conn),
        "label",
        "Invoice.customer",
        "writes_to",
        None,
    );
}

#[test]
fn test_v2_namedtuple_attrs_ctor() {
    let (_t, conn) = db("v2_namedtuple_attrs_ctor");
    let edges = query_edges(&conn);
    producer(&edges, "get_label", "Coord.label", "writes_to", None);
    assert_eq!(
        node_rows(&conn, "get_label", "postcondition"),
        ["choices=[\"pos\"]", "length=3", "nullability=1", "type=str"]
    );
    assert_eq!(
        override_rows(
            &conn,
            &edges,
            "make_point",
            "Point.label",
            "writes_to",
            None
        ),
        ["choices=[\"fixed\"]", "length=5", "nullability=0", "type=str"]
    );
}

#[test]
fn test_v2_narrowing() {
    let (_t, conn) = db("v2_narrowing");
    for f in ["safe_assert", "safe_truthy"] {
        assert_eq!(
            node_rows(&conn, f, "postcondition"),
            ["nullability=0"],
            "{f}"
        );
    }
    assert_eq!(
        node_rows(&conn, "unsafe", "postcondition"),
        ["nullability=1"]
    );
    producer(
        &query_edges(&conn),
        "unsafe",
        "Ticket.label",
        "writes_to",
        None,
    );
}

#[test]
fn test_v2_none_producers() {
    let (_t, conn) = db("v2_none_producers");
    let edges = query_edges(&conn);
    for (f, field) in [
        ("pick_ternary", "Bag.ternary"),
        ("pick_matched", "Bag.matched"),
        ("pick_attr", "Bag.attr"),
        ("pick_next", "Bag.nxt"),
        ("pick_latest", "Bag.latest"),
    ] {
        assert_eq!(
            node_rows(&conn, f, "postcondition"),
            ["nullability=1"],
            "{f}"
        );
        producer(&edges, f, field, "writes_to", None);
    }
}

#[test]
fn test_v2_per_param_requires() {
    let (_t, conn) = db("v2_per_param_requires");
    let edges = query_edges(&conn);
    producer(&edges, "make4dp", "combine", "flows_to", Some("base"));
    assert_eq!(
        override_rows(
            &conn,
            &edges,
            "make",
            "combine",
            "flows_to",
            Some("surcharge")
        ),
        ["nullability=0", "precision=4", "type=Decimal"]
    );
    let pre = node_rows(&conn, "combine", "precondition");
    assert!(
        pre.contains(&"base: precision=2 [ASSUMED]".to_string()),
        "{pre:?}"
    );
    assert!(
        pre.contains(&"surcharge: precision=4 [ASSUMED]".to_string()),
        "{pre:?}"
    );
}

#[test]
fn test_v2_pydantic_bounds() {
    let (_t, conn) = db("v2_pydantic_bounds");
    assert_eq!(
        node_rows(&conn, "Item.qty", "precondition"),
        ["nullability=0", "range=1.."]
    );
    assert_eq!(
        node_rows(&conn, "Item.pct", "precondition"),
        ["nullability=0", "range=0..100"]
    );
}

#[test]
fn test_v2_relative_import() {
    let (_t, conn) = db("v2_relative_import");
    producer(
        &query_edges(&conn),
        "boost",
        "Invoice.total",
        "writes_to",
        None,
    );
    let q = |n: &str| {
        query_nodes(&conn, "function")
            .into_iter()
            .find(|x| x.name == n)
            .and_then(|x| x.qualified_name)
    };
    assert_eq!(q("boost").as_deref(), Some("pkg.helpers.boost"));
}

#[test]
fn test_v2_self_local_attr_write() {
    let (_t, conn) = db("v2_self_local_attr_write");
    let edges = query_edges(&conn);
    assert_eq!(
        override_rows(&conn, &edges, "correct", "Reading.kwh", "writes_to", None),
        ["nullability=0", "precision=6", "type=Decimal"]
    );
    assert_eq!(
        override_rows(
            &conn,
            &edges,
            "Meter.bump",
            "Meter.total",
            "writes_to",
            None
        ),
        ["nullability=0", "precision=4", "type=Decimal"]
    );
}
