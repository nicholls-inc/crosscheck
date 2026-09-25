//! E2E extraction tests for round 6 (`docs/design/dataflow-v2.md`, "Round 6
//! (final verification pass)") and the `test_fixtures/r6_*` fixtures: the rows
//! behind the checker's verdicts, and what `expected.json` cannot show
//! (warnings that must not appear, edges that must not exist).

mod test_helpers;

use std::path::Path;

use tempfile::TempDir;
use test_helpers::*;

fn db(fixture: &str) -> (TempDir, rusqlite::Connection) {
    let tmp = TempDir::new().unwrap();
    let conn = extract_to_db(&Path::new("test_fixtures").join(fixture), &tmp);
    (tmp, conn)
}

fn edges_between<'e>(edges: &'e [EdgeRow], source: &str, target: &str) -> Vec<&'e EdgeRow> {
    edges
        .iter()
        .filter(|e| e.source_name == source && e.target_name == target)
        .collect()
}

fn rows_of(conn: &rusqlite::Connection, e: &EdgeRow) -> Vec<String> {
    render_rows(&query_edge_contracts(conn, e.id))
}

/// Return contract nodes: which functions get one, their requirements, and
/// the return-site edges.
#[test]
fn test_return_contract_nodes() {
    let (_t, conn) = db("r6_return_contract");
    let models: Vec<String> = query_nodes(&conn, "model").into_iter().map(|n| n.name).collect();
    for name in [
        "parse_kwh.<return>",
        "label.<return>",
        "unit.<return>",
        "as_text.<return>",
        "store.<return>",
        "Source.name.<return>",
        "Base.code.<return>",
    ] {
        assert!(models.contains(&name.to_string()), "{name} in {models:?}");
    }
    // Optional, a type variable: no return contract.
    for name in ["maybe.<return>", "first.<return>"] {
        assert!(!models.contains(&name.to_string()), "{name}");
    }
    assert_eq!(
        node_rows(&conn, "parse_kwh.<return>", "precondition"),
        ["nullability=0", "type=Decimal"]
    );
    assert_eq!(node_rows(&conn, "store.<return>", "precondition"), ["nullability=0"]);
    let edges = query_edges(&conn);
    // parse_kwh: `return None` and the quantized value.
    let mut ret: Vec<Vec<String>> = edges_between(&edges, "parse_kwh", "parse_kwh.<return>")
        .into_iter()
        .map(|e| rows_of(&conn, e))
        .collect();
    ret.sort();
    assert_eq!(ret.len(), 2);
    assert!(ret.iter().any(|r| r.contains(&"nullability=1".to_string())), "{ret:?}");
    // label falls off the end: an implicit None return.
    assert_eq!(edges_between(&edges, "label", "label.<return>").len(), 2);
    // Stubs are declarations: no return edges.
    assert!(edges_between(&edges, "Source.name", "Source.name.<return>").is_empty());
    assert!(edges_between(&edges, "Base.code", "Base.code.<return>").is_empty());
    // Callers rely on the annotation: parse_kwh's postcondition is non-null.
    assert!(node_rows(&conn, "parse_kwh", "postcondition").contains(&"nullability=0".to_string()));
    // `return label(raw)`-style results come from the call-site node.
    let store_writes = edges_between(&edges, "label", "Reading.source");
    assert!(store_writes.iter().all(|e| e.source_is_site && !e.source_override));
}

/// One edge per alternative, at one site.
#[test]
fn test_alternatives_are_separate_edges() {
    let (_t, conn) = db("r6_alternatives");
    let edges = query_edges(&conn);
    let qty = edges_between(&edges, "apply", "StockItem.qty");
    assert_eq!(qty.len(), 2, "{qty:?}");
    assert_eq!(qty[0].site_line, qty[1].site_line);
    let mut rows: Vec<Vec<String>> = qty.iter().map(|e| rows_of(&conn, e)).collect();
    rows.sort();
    assert!(rows.iter().any(|r| r.contains(&"range=-1..-1".to_string())), "{rows:?}");
    let code = edges_between(&edges, "local_defs", "StockItem.code");
    let lengths: Vec<Vec<String>> = code.iter().map(|e| rows_of(&conn, e)).collect();
    assert_eq!(code.len(), 2, "{lengths:?}");
    assert!(lengths.iter().any(|r| r.contains(&"length=6".to_string())));
    assert!(lengths.iter().any(|r| r.contains(&"length=9".to_string())));
}

/// Dynamic writes: an edge without guarantees to every field; none inside a
/// forwarder the project calls.
#[test]
fn test_dynamic_writes() {
    let (_t, conn) = db("r6_dynamic_writes");
    let edges = query_edges(&conn);
    for (src, field) in [
        ("by_setattr", "Payment.amount"),
        ("by_setattr", "Payment.note"),
        ("by_unknown_dict", "Payment.amount"),
        ("by_unknown_dict", "Payment.note"),
    ] {
        let es = edges_between(&edges, src, field);
        assert_eq!(es.len(), 1, "{src} -> {field}");
        assert!(es[0].source_override && rows_of(&conn, es[0]).is_empty(), "{src} -> {field}");
    }
    assert!(edges_between(&edges, "forwarder", "Payment.amount").is_empty());
    // `__dict__.update(amount=...)` is a write of that value.
    let d = edges_between(&edges, "by_dict_update", "Payment.amount");
    assert!(d.iter().any(|e| rows_of(&conn, e).contains(&"precision=3".to_string())));
}

/// A `None`-default parameter of a function with a project decorator has
/// unknown nullability; without one it is nullable.
#[test]
fn test_decorator_injected_parameters() {
    let (_t, conn) = db("r6_decorator_injected");
    let edges = query_edges(&conn);
    let flow = |src: &str| {
        let es: Vec<&EdgeRow> = edges
            .iter()
            .filter(|e| {
                e.source_name == src
                    && e.target_name == "record"
                    && e.relationship == "flows_to"
                    && e.target_param.as_deref() == Some("db_session")
                    && !e.target_is_site
            })
            .collect();
        assert_eq!(es.len(), 1, "{src}");
        rows_of(&conn, es[0])
    };
    assert!(!flow("log_event").iter().any(|r| r.starts_with("nullability")));
    assert!(!flow("log_event_typed").iter().any(|r| r.starts_with("nullability")));
    assert_eq!(flow("plain"), ["nullability=1"]);
    assert_eq!(flow("cached"), ["nullability=1"]);
}

/// Class hierarchy analysis: one call-site node per dispatch target.
#[test]
fn test_dispatch_call_sites() {
    let (_t, conn) = db("r6_dispatch");
    let mut stmt = conn
        .prepare("SELECT qualified_name FROM nodes WHERE is_call_site = 1 ORDER BY qualified_name")
        .unwrap();
    let sites: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    // `tariff.price(net)` in quote_with (line 46): every Tariff override.
    for callee in ["bug.Tariff.price", "bug.PreciseTariff.price", "bug.Discounted.price"] {
        assert!(sites.contains(&format!("{callee}@bug.py:46")), "{callee} in {sites:?}");
    }
    // `Precise().price(net)`: Precise's (inherited) price only.
    assert!(sites.contains(&"bug.Engine.price@bug.py:31".to_string()), "{sites:?}");
    assert!(!sites.iter().any(|s| s.ends_with("@bug.py:31") && s != "bug.Engine.price@bug.py:31"));
}

/// Field reads carry the field's declared contracts; a literal None into a
/// None-accepting field has no edge.
#[test]
fn test_field_reads_and_none_writes() {
    let (_t, conn) = db("r6_field_reads");
    let edges = query_edges(&conn);
    let rows = |target: &str| {
        let es = edges_between(&edges, "audit", target);
        assert_eq!(es.len(), 1, "{target}");
        rows_of(&conn, es[0])
    };
    assert!(rows("AuditLog.object_ref").contains(&"length=12".to_string()));
    assert!(rows("AuditLog.short_ref").contains(&"length=3".to_string()));
    assert!(rows("AuditLog.amount").contains(&"precision=2".to_string()));
    assert!(rows("AuditLog.points").contains(&"range=1..".to_string()), "{:?}", rows("AuditLog.points"));

    let (_t2, conn2) = db("r6_value_bounds");
    let edges2 = query_edges(&conn2);
    assert!(edges_between(&edges2, "welcome", "Coupon.percent_off").is_empty());
    assert_eq!(edges_between(&edges2, "welcome", "Coupon.code").len(), 1);
}

/// Loop variables over querysets, typed collections and bulk-updated lists.
#[test]
fn test_queryset_loop_writes() {
    let (_t, conn) = db("r6_queryset_loops");
    let edges = query_edges(&conn);
    for (src, field) in [
        ("bulk", "Payment.amount"),
        ("over_queryset", "Payment.ref"),
        ("over_typed", "Payment.amount"),
        ("over_queryset_ok", "Payment.amount"),
        ("over_queryset_ok", "Payment.ref"),
    ] {
        assert!(!edges_between(&edges, src, field).is_empty(), "{src} -> {field}");
    }
}

/// Dict locals, `dict(k=v)`, returned dicts, and dicts passed to forwarders.
#[test]
fn test_dict_splats() {
    let (_t, conn) = db("r6_dict_splats");
    let edges = query_edges(&conn);
    for src in ["local_to_method", "returned_dict", "dict_call", "returned_to_forwarder"] {
        let es = edges_between(&edges, src, "Payment.amount");
        assert!(
            es.iter().any(|e| rows_of(&conn, e).contains(&"precision=3".to_string())),
            "{src}: {es:?}"
        );
    }
    // `data = {"amount": three(amount)}; create(data)`: from three's call site.
    let three: Vec<&EdgeRow> = edges_between(&edges, "three", "Payment.amount");
    assert!(three.iter().any(|e| e.source_is_site && !e.source_override));
}
