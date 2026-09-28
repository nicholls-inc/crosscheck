//! E2E extraction tests for the round-5 interface (`docs/design/dataflow-v2.md`,
//! "Round 5") and the `test_fixtures/r5_*` fixtures: rows the checker cannot
//! yet show (decimal columns, call-site flags), and noise that `expected.json`
//! does not compare (warnings).

mod test_helpers;

use std::path::Path;

use crosscheck_contracts::extractor::{self, ExtractOptions};
use tempfile::TempDir;
use test_helpers::*;

fn db(fixture: &str) -> (TempDir, rusqlite::Connection) {
    let tmp = TempDir::new().unwrap();
    let conn = extract_to_db(&Path::new("test_fixtures").join(fixture), &tmp);
    (tmp, conn)
}

/// `(node, role, min micros, max micros, min decimal, max decimal)` of every range row.
type RangeRow = (String, String, Option<i64>, Option<i64>, Option<String>, Option<String>);

fn range_rows(conn: &rusqlite::Connection) -> Vec<RangeRow> {
    let mut stmt = conn
        .prepare(
            "SELECT n.name, c.contract_role, c.param_min_micros, c.param_max_micros,
                    c.param_min_decimal, c.param_max_decimal
             FROM contracts c JOIN nodes n ON n.id = c.node_id
             WHERE c.constraint_type = 'range' ORDER BY c.id",
        )
        .unwrap();
    stmt.query_map([], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))
    })
    .unwrap()
    .map(Result::unwrap)
    .collect()
}

/// Exact decimal strings on every range row; micros NULL past `i64`.
#[test]
fn test_decimal_columns_big_bounds() {
    let (_t, conn) = db("r5_big_bounds");
    let rows = range_rows(&conn);
    let n = rows.iter().find(|r| r.0 == "Big.n").unwrap();
    assert_eq!((n.3, n.5.as_deref()), (None, Some("10000000000000")));
    let m = rows.iter().find(|r| r.0 == "Big.m").unwrap();
    assert_eq!((m.2, m.4.as_deref()), (None, Some("-10000000000000")));
    let bad: Vec<&RangeRow> = rows.iter().filter(|r| r.0 == "bad").collect();
    assert_eq!(bad.len(), 2);
    assert!(bad.iter().all(|r| r.2.is_none() && r.3.is_none()));
    assert_eq!(bad[0].5.as_deref(), Some("20000000000000"));
    assert_eq!(bad[1].4.as_deref(), Some("-20000000000000"));
    // Within i64 micros: both columns.
    let ok = rows.iter().find(|r| r.0 == "ok").unwrap();
    assert_eq!((ok.2, ok.4.as_deref()), (Some(5_000_000), Some("5")));
}

#[test]
fn test_decimal_columns_fine_bounds() {
    let (_t, conn) = db("r5_fine_bounds");
    let rows = range_rows(&conn);
    let x = rows.iter().find(|r| r.0 == "P.x").unwrap();
    // A requirement's micros round to the stricter side; the decimal is exact.
    assert_eq!((x.3, x.5.as_deref()), (Some(123_456), Some("0.1234567")));
    let y = rows.iter().find(|r| r.0 == "P.y").unwrap();
    assert_eq!((y.2, y.4.as_deref()), (Some(123_457), Some("0.1234567")));
    // Exponent notation is normalised.
    assert!(rows
        .iter()
        .any(|r| r.0 == "inside" && r.4.as_deref() == Some("0.15")));
}

#[test]
fn test_is_call_site() {
    let (_t, conn) = db("r5_context_ident");
    let mut stmt = conn
        .prepare("SELECT qualified_name, is_call_site FROM nodes WHERE kind = 'function'")
        .unwrap();
    let rows: Vec<(String, i64)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert!(!rows.is_empty());
    for (q, site) in &rows {
        assert_eq!(*site == 1, q.contains('@'), "{q}");
    }
    assert!(rows.iter().any(|(_, s)| *s == 1));
}

/// Nullability of every override row written into `fields` (by display name).
fn override_nullability(conn: &rusqlite::Connection, target_prefix: &str) -> Vec<(String, Option<i64>)> {
    let mut stmt = conn
        .prepare(
            "SELECT t.name, (SELECT c.param_nullable FROM contracts c
                             WHERE c.edge_id = e.id AND c.constraint_type = 'nullability')
             FROM edges e JOIN nodes t ON t.id = e.target_node_id
             WHERE e.relationship = 'writes_to' AND e.source_override = 1",
        )
        .unwrap();
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .filter(|(t, _): &(String, Option<i64>)| t.starts_with(target_prefix))
        .collect()
}

/// Round 5 N10: values that cannot be None are written as non-null.
#[test]
fn test_non_null_by_construction() {
    let (_t, conn) = db("r5_non_null");
    let rows = override_nullability(&conn, "Summary.");
    assert_eq!(rows.len(), 7, "{rows:?}");
    for (field, nullable) in &rows {
        assert_eq!(*nullable, Some(0), "{field}");
    }
    let wallet = override_nullability(&conn, "Wallet.");
    let get = |f: &str| wallet.iter().filter(|(t, _)| t == f).map(|(_, n)| *n).collect::<Vec<_>>();
    // F("points") + 1 and a read of a non-null field; an Optional field read; min(default=None).
    assert_eq!(get("Wallet.points"), [Some(0), Some(0), Some(1)]);
    assert_eq!(get("Wallet.label"), [Some(0), Some(1), Some(0)]);
}

/// Round 5 N2: a value overwritten or narrowed before the save is written
/// only as it is saved.
#[test]
fn test_flow_sensitive_attribute_writes() {
    let (_t, conn) = db("r5_attr_flow");
    let edges = query_edges(&conn);
    let label: Vec<&EdgeRow> = edges
        .iter()
        .filter(|e| e.target_name == "Card.label" && e.relationship == "writes_to")
        .collect();
    // long_label: only "toolong" reaches the save.
    assert_eq!(label.len(), 1, "{label:?}");
    let ops_label: Vec<&EdgeRow> = edges
        .iter()
        .filter(|e| e.target_name == "CardOps.label" && e.relationship == "writes_to")
        .collect();
    // relabel: only "short".
    assert_eq!(ops_label.len(), 1);
    let rows = query_edge_contracts(&conn, ops_label[0].id);
    assert!(rows.iter().any(|r| r.constraint_type == "length" && r.param_max_length == Some(5)));
    // init_type / lock / truthy: the lookup result is saved non-null.
    let narrowed: Vec<&EdgeRow> = edges
        .iter()
        .filter(|e| e.target_name == "CardOps.card_type" && e.source_name == "lookup")
        .collect();
    assert_eq!(narrowed.len(), 3);
    for e in narrowed {
        assert!(e.source_override);
        let rows = query_edge_contracts(&conn, e.id);
        assert!(rows
            .iter()
            .any(|r| r.constraint_type == "nullability" && r.param_nullable == Some(0)));
    }
}

/// Round 5 N11: `--exclude` globs over paths relative to the app root.
#[test]
fn test_exclude_globs() {
    let tmp = TempDir::new().unwrap();
    let app = tmp.path().join("app");
    std::fs::create_dir_all(app.join("shop/tests")).unwrap();
    std::fs::write(
        app.join("shop/models.py"),
        "from django.db import models\nclass P(models.Model):\n    name = models.CharField(max_length=3)\n",
    )
    .unwrap();
    std::fs::write(
        app.join("shop/tests/test_p.py"),
        "from shop.models import P\ndef test_long():\n    P(name='toolong')\n",
    )
    .unwrap();
    let writes = |exclude: Vec<String>| {
        let db = tmp.path().join("c.sqlite");
        let options = ExtractOptions {
            exclude,
            ..Default::default()
        };
        let path = extractor::extract_with(&app, None, "4.2", Some(&db), &options).unwrap();
        let conn = rusqlite::Connection::open(path).unwrap();
        query_edges(&conn)
            .into_iter()
            .filter(|e| e.relationship == "writes_to")
            .count()
    };
    assert_eq!(writes(vec![]), 1);
    assert_eq!(writes(vec!["**/tests/**".to_string()]), 0);
    assert_eq!(writes(vec!["shop/tests".to_string()]), 0);
    assert_eq!(writes(vec!["**/test_*.py".to_string()]), 0);
    assert_eq!(writes(vec!["tests/**".to_string()]), 1, "anchored at the app root");
}
