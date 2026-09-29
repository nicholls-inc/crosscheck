//! E2E extraction tests for fixes from the pr-swarm review of PR #3
//! (`test_fixtures/r8_*`): guarantees that must not be tighter than the value
//! written. `expected.json` can only show the resulting warnings; these pin
//! the rows behind them.

mod test_helpers;

use std::path::Path;

use tempfile::TempDir;
use test_helpers::*;

fn rows(fixture: &str, source: &str, target: &str) -> Vec<String> {
    let tmp = TempDir::new().unwrap();
    let conn = extract_to_db(&Path::new("test_fixtures").join(fixture), &tmp);
    let edges = query_edges(&conn);
    override_rows(&conn, &edges, source, target, "writes_to", None)
}

/// A FloatField read is a binary float: a sum of two reads gets no exact
/// range bound (0.1 + 0.2 > 0.3).
#[test]
fn test_float_field_sum_has_no_exact_bound() {
    let r = rows("r8_float_and_singleton", "combine", "Total.total");
    assert!(!r.iter().any(|row| row.starts_with("range")), "{r:?}");
}

/// `max([x])` and `max(xs)` return one element uncompared, so they may be
/// None; `max([x, y])` compares, so it is not.
#[test]
fn test_max_of_one_element_may_be_none() {
    for source in ["peak", "peak_literal"] {
        let r = rows("r8_float_and_singleton", source, "Total.n");
        assert!(!r.iter().any(|row| row.starts_with("nullability")), "{source}: {r:?}");
    }
    let r = rows("r8_float_and_singleton", "larger", "Total.n");
    assert!(r.contains(&"nullability=0".to_string()), "{r:?}");
}
