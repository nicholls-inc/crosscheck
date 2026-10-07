//! E2E tests for translation (TB-1.12): the Lean checker exits 2 on a database
//! with a row it cannot translate, instead of dropping the row and checking
//! the rest. Each case extracts the bug1 fixture (two precision errors, exit
//! 1), breaks one row with SQL, and runs the built checker on the result.
//! Several of the broken databases exited 0 before TB-1.12.
//!
//! Skipped (with an explanation) if the Lean checker binary has not been built.

mod test_helpers;

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;
use test_helpers::extract_to_db;

fn lean_checker_binary() -> PathBuf {
    PathBuf::from("prover/.lake/build/bin/contract-graph-checker")
}

/// Extract bug1, apply `sql`, and run the checker: (exit code, stderr).
fn check_broken(sql: &str) -> Option<(i32, String)> {
    let checker = lean_checker_binary();
    if !checker.is_file() {
        eprintln!(
            "SKIP: Lean checker binary not found at {} (run `cd prover && lake build` first)",
            checker.display()
        );
        return None;
    }
    let tmp = TempDir::new().unwrap();
    let conn = extract_to_db(Path::new("test_fixtures/bug1"), &tmp);
    // The extractor writes with foreign keys off (`src/db.rs`), so a database
    // can hold ids that name no row.
    conn.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();
    conn.execute_batch(sql).unwrap();
    drop(conn);
    let out = Command::new(&checker)
        .arg(tmp.path().join("contracts.sqlite"))
        .output()
        .expect("failed to run the checker");
    Some((out.status.code().unwrap(), String::from_utf8_lossy(&out.stderr).into_owned()))
}

fn assert_rejected(sql: &str, reason: &str) {
    let Some((code, stderr)) = check_broken(sql) else { return };
    assert_eq!(code, 2, "expected exit 2 for `{sql}`, stderr:\n{stderr}");
    assert!(stderr.contains("cannot translate the database"), "stderr:\n{stderr}");
    assert!(stderr.contains(reason), "expected `{reason}` in stderr:\n{stderr}");
}

#[test]
fn intact_database_still_reports_its_errors() {
    let Some((code, stderr)) = check_broken("") else { return };
    assert_eq!(code, 1, "stderr:\n{stderr}");
}

#[test]
fn edge_to_a_missing_node_is_rejected() {
    assert_rejected(
        "DELETE FROM nodes WHERE kind = 'model' AND name LIKE 'EnergyRecord.%';",
        "which does not exist",
    );
}

#[test]
fn contract_row_of_a_missing_node_is_rejected() {
    assert_rejected(
        "UPDATE contracts SET node_id = 9999 WHERE contract_role = 'precondition' \
         AND constraint_type = 'precision';",
        "names node 9999, which does not exist",
    );
}

#[test]
fn contract_row_of_a_missing_edge_is_rejected() {
    assert_rejected(
        "UPDATE contracts SET edge_id = edge_id + 1000 WHERE edge_id IS NOT NULL;",
        "which does not exist",
    );
}

#[test]
fn per_edge_row_on_an_edge_without_override_is_rejected() {
    assert_rejected("UPDATE edges SET source_override = 0;", "has no source_override");
}

#[test]
fn unknown_relationship_is_rejected() {
    assert_rejected(
        "PRAGMA ignore_check_constraints = ON; UPDATE edges SET relationship = 'writes';",
        "unknown relationship 'writes'",
    );
}

#[test]
fn unknown_node_kind_is_rejected() {
    assert_rejected(
        "PRAGMA ignore_check_constraints = ON; UPDATE nodes SET kind = 'Model' WHERE kind = 'model';",
        "unknown kind 'Model'",
    );
}

#[test]
fn unknown_constraint_type_is_rejected() {
    assert_rejected(
        "PRAGMA ignore_check_constraints = ON; \
         UPDATE contracts SET constraint_type = 'decimal_places' WHERE constraint_type = 'precision';",
        "unknown constraint_type",
    );
}

#[test]
fn unknown_contract_role_is_rejected() {
    assert_rejected(
        "PRAGMA ignore_check_constraints = ON; \
         UPDATE contracts SET contract_role = 'guarantee' WHERE contract_role = 'postcondition';",
        "unknown contract_role 'guarantee'",
    );
}

#[test]
fn unknown_verification_level_is_rejected() {
    assert_rejected(
        "PRAGMA ignore_check_constraints = ON; UPDATE contracts SET verification_level = 'GUESSED';",
        "unknown verification_level 'GUESSED'",
    );
}

#[test]
fn row_without_its_value_is_rejected() {
    assert_rejected(
        "UPDATE contracts SET param_decimal_places = NULL WHERE contract_role = 'precondition' \
         AND constraint_type = 'precision';",
        "no value and no dependent_expr",
    );
}
