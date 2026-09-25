//! E2E extraction tests for the `test_fixtures/limits_*` fixtures (data-flow
//! model v2, `docs/design/dataflow-v2.md`).
//!
//! Each test asserts the database rows the fixture's `expected.json` verdict
//! depends on: node names, edges with `target_param` / `source_override`,
//! and the override rows of each edge. The verdicts themselves are checked
//! by `scripts/check-fixtures.sh` against the Lean checker.

mod test_helpers;

use std::path::Path;
use tempfile::TempDir;
use test_helpers::*;

fn db(fixture: &str) -> (TempDir, rusqlite::Connection) {
    let tmp = TempDir::new().unwrap();
    let conn = extract_to_db(&Path::new("test_fixtures").join(fixture), &tmp);
    (tmp, conn)
}

fn names(conn: &rusqlite::Connection, kind: &str) -> Vec<String> {
    let mut v: Vec<String> = query_nodes(conn, kind)
        .into_iter()
        .map(|n| n.name)
        .collect();
    v.sort();
    v
}

/// `source -rel-> target [param] [override]` for every edge, sorted.
fn edge_list(edges: &[EdgeRow]) -> Vec<String> {
    let mut v: Vec<String> = edges
        .iter()
        .map(|e| {
            let mut s = format!("{} -{}-> {}", e.source_name, e.relationship, e.target_name);
            if let Some(p) = &e.target_param {
                s.push_str(&format!(" [{p}]"));
            }
            if e.source_override {
                s.push_str(" [override]");
            }
            s
        })
        .collect();
    v.sort();
    v
}

fn sorted(v: &[&str]) -> Vec<String> {
    let mut v: Vec<String> = v.iter().map(|s| s.to_string()).collect();
    v.sort();
    v
}

const INVOICE_FIELDS: [&str; 3] = ["Invoice.customer", "Invoice.tax", "Invoice.total"];

#[test]
fn test_limits_attr_assign() {
    let (_t, conn) = db("limits_attr_assign");
    let edges = query_edges(&conn);
    assert_eq!(
        edge_list(&edges),
        ["update -writes_to-> Invoice.total [override]"]
    );
    assert_eq!(
        override_rows(&conn, &edges, "update", "Invoice.total", "writes_to", None),
        ["nullability=0", "precision=4", "type=Decimal"]
    );
    assert!(node_rows(&conn, "Invoice.total", "precondition").contains(&"precision=2".to_string()));
}

#[test]
fn test_limits_django_save() {
    let (_t, conn) = db("limits_django_save");
    let edges = query_edges(&conn);
    // `r.kwh = ...` is the write; `r.save()` adds nothing.
    assert_eq!(
        edge_list(&edges),
        ["correct -writes_to-> Reading.kwh [override]"]
    );
    assert_eq!(
        override_rows(&conn, &edges, "correct", "Reading.kwh", "writes_to", None),
        ["nullability=0", "precision=6", "type=Decimal"]
    );
    assert!(node_rows(&conn, "Reading.kwh", "precondition").contains(&"precision=3".to_string()));
}

#[test]
fn test_limits_kwargs() {
    let (_t, conn) = db("limits_kwargs");
    let edges = query_edges(&conn);
    assert_eq!(
        edge_list(&edges),
        sorted(&[
            "make -writes_to-> Invoice.customer [override]",
            "make -writes_to-> Invoice.tax [override]",
            "make -writes_to-> Invoice.total [override]",
        ])
    );
    assert_eq!(
        override_rows(&conn, &edges, "make", "Invoice.total", "writes_to", None),
        ["nullability=0", "precision=4", "type=Decimal"]
    );
}

#[test]
fn test_limits_qualified_call() {
    let (_t, conn) = db("limits_qualified_call");
    let edges = query_edges(&conn);
    // `helpers.label(first, last)` resolves through `import helpers`.
    let e = the_edge(&edges, "label", "Invoice.customer", "writes_to", None);
    assert!(!e.source_override);
    assert!(node_rows(&conn, "label", "postcondition").contains(&"length=64 [ASSUMED]".to_string()));
    assert_eq!(
        override_rows(&conn, &edges, "make", "label", "flows_to", Some("first")),
        ["nullability=0", "type=str"]
    );
    the_edge(&edges, "make", "label", "calls", None);
    let label = query_nodes(&conn, "function")
        .into_iter()
        .find(|n| n.name == "label")
        .unwrap();
    assert_eq!(label.qualified_name.as_deref(), Some("helpers.label"));
}

#[test]
fn test_limits_literal_none() {
    let (_t, conn) = db("limits_literal_none");
    let edges = query_edges(&conn);
    assert_eq!(
        override_rows(&conn, &edges, "make", "Invoice.total", "writes_to", None),
        ["nullability=1"]
    );
    // Non-None literals are non-null.
    assert_eq!(
        override_rows(&conn, &edges, "make", "Invoice.customer", "writes_to", None),
        ["length=1", "nullability=0", "type=str"]
    );
    assert_eq!(
        override_rows(&conn, &edges, "make", "Invoice.tax", "writes_to", None),
        ["nullability=0", "precision=0", "type=Decimal"]
    );
}

#[test]
fn test_limits_dict_get() {
    let (_t, conn) = db("limits_dict_get");
    let edges = query_edges(&conn);
    let e = the_edge(&edges, "price_for", "Invoice.total", "writes_to", None);
    assert!(!e.source_override);
    // `PRICES.get(sku)` is a None producer; price_for has no return annotation.
    assert_eq!(
        node_rows(&conn, "price_for", "postcondition"),
        ["nullability=1"]
    );
    assert!(
        node_rows(&conn, "Invoice.total", "precondition").contains(&"nullability=0".to_string())
    );
}

#[test]
fn test_limits_arith_unknown() {
    let (_t, conn) = db("limits_arith_unknown");
    let edges = query_edges(&conn);
    // 2dp * rate: precision unknown (no row), so the checker warns instead of passing silently.
    assert_eq!(
        override_rows(&conn, &edges, "make", "Invoice.total", "writes_to", None),
        ["nullability=0", "type=Decimal"]
    );
    assert_eq!(
        override_rows(&conn, &edges, "make", "Invoice.tax", "writes_to", None),
        ["nullability=0", "precision=0", "type=Decimal"]
    );
}

#[test]
fn test_limits_lower_bound() {
    let (_t, conn) = db("limits_lower_bound");
    let edges = query_edges(&conn);
    assert!(!the_edge(&edges, "adjust", "Stock.qty", "writes_to", None).source_override);
    assert_eq!(
        node_rows(&conn, "Stock.qty", "precondition"),
        ["nullability=0", "range=0..1000", "type=int"]
    );
    // Docstring `result >= -50` / `result <= 500`: lower bound in param_min_value.
    assert_eq!(
        node_rows(&conn, "adjust", "postcondition"),
        [
            "nullability=0",
            "precision=0",
            "range=-50.. [ASSUMED]",
            "range=..500 [ASSUMED]",
            "type=int",
        ]
    );
}

#[test]
fn test_limits_name_clash() {
    let (_t, conn) = db("limits_name_clash");
    assert_eq!(
        names(&conn, "model"),
        [
            "billing.records.Invoice.total",
            "shop.records.Invoice.total"
        ]
    );
    assert_eq!(
        names(&conn, "function"),
        ["billing.code.make", "shop.code.make"]
    );
    let edges = query_edges(&conn);
    assert_eq!(
        edge_list(&edges),
        [
            "billing.code.make -writes_to-> billing.records.Invoice.total [override]",
            "shop.code.make -writes_to-> shop.records.Invoice.total [override]",
        ]
    );
    for m in ["billing", "shop"] {
        assert_eq!(
            override_rows(
                &conn,
                &edges,
                &format!("{m}.code.make"),
                &format!("{m}.records.Invoice.total"),
                "writes_to",
                None
            ),
            ["nullability=0", "precision=4", "type=Decimal"]
        );
    }
    let places = |n: &str| query_contract_by_type(&conn, n, "precision")[0].param_decimal_places;
    assert_eq!(places("billing.records.Invoice.total"), Some(2));
    assert_eq!(places("shop.records.Invoice.total"), Some(6));
}

#[test]
fn test_limits_per_field() {
    let (_t, conn) = db("limits_per_field");
    let edges = query_edges(&conn);
    assert_eq!(
        override_rows(&conn, &edges, "make", "Invoice.total", "writes_to", None),
        ["nullability=0", "precision=2", "type=Decimal"]
    );
    assert_eq!(
        override_rows(&conn, &edges, "make", "Invoice.tax", "writes_to", None),
        ["nullability=0", "precision=4", "type=Decimal"]
    );
    // The function's own postconditions say nothing about precision.
    assert_eq!(node_rows(&conn, "make", "postcondition"), ["nullability=0"]);
}

#[test]
fn test_limits_per_argument() {
    let (_t, conn) = db("limits_per_argument");
    assert_eq!(names(&conn, "model"), INVOICE_FIELDS);
    assert_eq!(
        names(&conn, "function"),
        ["combine", "make", "make_bad", "with_tax"]
    );
    let edges = query_edges(&conn);
    assert_eq!(
        edge_list(&edges),
        sorted(&[
            "combine -writes_to-> Invoice.total",
            "make -writes_to-> Invoice.tax [override]",
            "make -writes_to-> Invoice.customer [override]",
            "make -calls-> combine",
            "make -flows_to-> combine [amount] [override]",
            "with_tax -flows_to-> combine [rate]",
            "make -calls-> with_tax",
            "make -flows_to-> with_tax [a] [override]",
            "make_bad -writes_to-> Invoice.tax [override]",
            "make_bad -writes_to-> Invoice.customer [override]",
            "make_bad -calls-> combine",
            "with_tax -flows_to-> combine [amount]",
            "make_bad -flows_to-> combine [rate] [override]",
            "make_bad -calls-> with_tax",
            "make_bad -flows_to-> with_tax [a] [override]",
        ])
    );
    assert_eq!(
        override_rows(&conn, &edges, "make", "combine", "flows_to", Some("amount")),
        ["nullability=0", "precision=2", "type=Decimal"]
    );
    assert_eq!(
        override_rows(
            &conn,
            &edges,
            "make_bad",
            "combine",
            "flows_to",
            Some("rate")
        ),
        ["nullability=0", "precision=2", "type=Decimal"]
    );
    // make has two parameters: b's precision is unknown, not guessed.
    assert_eq!(
        override_rows(&conn, &edges, "make", "with_tax", "flows_to", Some("a")),
        ["nullability=0", "type=Decimal"]
    );
    // Per-parameter preconditions: only `amount` requires 2dp.
    assert_eq!(
        node_rows(&conn, "combine", "precondition"),
        [
            "amount: nullability=0",
            "amount: precision=2 [ASSUMED]",
            "amount: type=Decimal",
            "rate: nullability=0",
            "rate: type=Decimal",
        ]
    );
    // combine returns a parameter of a two-parameter function: no precision,
    // so `combine -> Invoice.total` warns.
    assert_eq!(
        node_rows(&conn, "combine", "postcondition"),
        ["nullability=0", "type=Decimal"]
    );
    assert_eq!(
        node_rows(&conn, "with_tax", "postcondition"),
        ["nullability=0", "precision=4", "type=Decimal"]
    );
}

#[test]
fn test_limits_interprocedural_none() {
    let (_t, conn) = db("limits_interprocedural_none");
    let edges = query_edges(&conn);
    // p = price(sku); Invoice(total=p)
    assert!(!the_edge(&edges, "price", "Invoice.total", "writes_to", None).source_override);
    // find returns PRICES.get(...); price returns find(...): nullable by the fixpoint.
    assert_eq!(node_rows(&conn, "find", "postcondition"), ["nullability=1"]);
    assert_eq!(
        node_rows(&conn, "price", "postcondition"),
        ["nullability=1"]
    );
    assert_eq!(
        override_rows(&conn, &edges, "make", "price", "flows_to", Some("sku")),
        ["nullability=0", "type=str"]
    );
}

#[test]
fn test_limits_methods() {
    let (_t, conn) = db("limits_methods");
    assert_eq!(names(&conn, "function"), ["Pricer.make", "Pricer.rounded"]);
    let edges = query_edges(&conn);
    // `self.rounded(a)` resolves to the method; `a` binds the parameter after `self`.
    assert!(
        !the_edge(&edges, "Pricer.rounded", "Invoice.total", "writes_to", None).source_override
    );
    the_edge(&edges, "Pricer.make", "Pricer.rounded", "calls", None);
    assert_eq!(
        override_rows(
            &conn,
            &edges,
            "Pricer.make",
            "Pricer.rounded",
            "flows_to",
            Some("a")
        ),
        ["nullability=0", "precision=input_precision", "type=Decimal"]
    );
    assert_eq!(
        node_rows(&conn, "Pricer.rounded", "postcondition"),
        ["nullability=0", "precision=4", "type=Decimal"]
    );
    assert_eq!(
        node_rows(&conn, "Pricer.rounded", "precondition"),
        ["a: nullability=0", "a: type=Decimal"]
    );
}

#[test]
fn test_limits_narrowing() {
    let (_t, conn) = db("limits_narrowing");
    let edges = query_edges(&conn);
    let customer = |f: &str| override_rows(&conn, &edges, f, "Invoice.customer", "writes_to", None);
    assert_eq!(
        customer("make"),
        ["nullability=0", "type=str"],
        "narrowed by `if code is None: raise`"
    );
    assert_eq!(
        customer("make_guarded"),
        ["nullability=0", "type=str"],
        "inside `if code is not None:`"
    );
    assert_eq!(customer("make_unsafe"), ["nullability=1", "type=str"]);
    assert_eq!(
        node_rows(&conn, "make_unsafe", "precondition"),
        ["code: nullability=1", "code: type=str"]
    );
}
