//! E2E test for `contracts check --format text`: runs the built CLI binary
//! end-to-end (extractor + Lean checker) on the transitive fixture and
//! checks the human-readable report shape described in
//! `docs/design/system-design.md` section 5.3.
//!
//! Skipped (with an explanation) if either the CLI binary or the Lean
//! checker binary has not been built yet.

use std::path::PathBuf;
use std::process::Command;

/// Cargo builds the crate's own binary before running this test and
/// exposes its path via this env var.
fn cli_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_crosscheck-contracts"))
}

fn lean_checker_binary() -> PathBuf {
    PathBuf::from("prover/.lake/build/bin/contract-graph-checker")
}

#[test]
fn text_format_on_transitive_fixture() {
    let cli = cli_binary();

    let checker = lean_checker_binary();
    if !checker.is_file() {
        eprintln!(
            "SKIP text_format_on_transitive_fixture: Lean checker binary not found at {} \
             (run `cd prover && lake build` first)",
            checker.display()
        );
        return;
    }

    let output = Command::new(&cli)
        .args([
            "contracts",
            "check",
            "test_fixtures/transitive/",
            "--lean-checker",
        ])
        .arg(&checker)
        .arg("--format")
        .arg("text")
        .output()
        .expect("failed to run crosscheck-contracts");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Header with counts.
    assert!(
        stdout.contains("CONTRACTS CHECKED:"),
        "missing contracts header, got:\n{stdout}"
    );
    assert!(stdout.contains("EDGES CHECKED:"), "got:\n{stdout}");
    assert!(stdout.contains("STATES CHECKED:"), "got:\n{stdout}");

    // The transitive fixture's headline finding: a 3-hop error whose
    // failing hop is not the first edge, so it must carry the
    // "invisible to pairwise checking" note.
    assert!(stdout.contains("ERROR"), "got:\n{stdout}");
    assert!(
        stdout.contains("Path: compute_offpeak \u{2192} split_energy \u{2192} EnergyRecord.energy"),
        "got:\n{stdout}"
    );
    assert!(
        stdout.contains("Failing hop: split_energy \u{2192} EnergyRecord.energy"),
        "got:\n{stdout}"
    );
    assert!(
        stdout.contains("Note: invisible to pairwise checking."),
        "got:\n{stdout}"
    );
    assert!(
        stdout.contains("Path verification level:"),
        "got:\n{stdout}"
    );
    assert!(stdout.contains("Suggestion:"), "got:\n{stdout}");

    // RESULT summary line, and the process exit code must match the
    // checker's semantic exit code (fixture is inconsistent -> 1).
    assert!(
        stdout.contains("RESULT: 1 error, 1 warning, 0 unverified. Exit code 1."),
        "got:\n{stdout}"
    );
    assert_eq!(output.status.code(), Some(1));
}

/// Run the CLI on `fixture` with `extra` arguments; `None` when the Lean
/// checker has not been built.
fn run_fixture(fixture: &str, db: &std::path::Path, extra: &[&str]) -> Option<(String, Option<i32>)> {
    let checker = lean_checker_binary();
    if !checker.is_file() {
        eprintln!("SKIP: Lean checker binary not found at {}", checker.display());
        return None;
    }
    let output = Command::new(cli_binary())
        .args(["contracts", "check", fixture, "--lean-checker"])
        .arg(&checker)
        .arg("--output-db")
        .arg(db)
        .args(extra)
        .output()
        .expect("failed to run crosscheck-contracts");
    Some((String::from_utf8_lossy(&output.stdout).into_owned(), output.status.code()))
}

/// The checker's own missing-guarantee warnings on `r6_alternatives` are
/// hidden from the default text report, counted in the coverage section
/// and the RESULT line, and listed under `--warnings` (UC-1 to UC-9). The
/// JSON count pins the checker's wording that UC-1 reads.
#[test]
fn missing_guarantees_are_coverage_on_a_real_run() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db = tmp.path().join("c.sqlite");
    let fixture = "test_fixtures/r6_alternatives/";
    let Some((json, _)) = run_fixture(fixture, &db, &[]) else { return };
    let json: serde_json::Value = serde_json::from_str(&json).expect("JSON output");
    let missing: Vec<&serde_json::Value> = json["results"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["suggestion"].as_str().unwrap().contains(" postcondition for the value it passes to "))
        .collect();
    assert_eq!(missing.len(), 2, "{json}");
    for r in &missing {
        assert_eq!(r["severity"], "warning", "{r}");
        assert!(r["source_guarantee"].as_str().unwrap().ends_with(" (unspecified)"), "{r}");
    }

    let (text, code) = run_fixture(fixture, &db, &["--format", "text"]).unwrap();
    assert_eq!(code, Some(1));
    assert!(text.contains("EDGES CHECKED: 24\n"), "{text}");
    assert!(!text.contains("UNVERIFIED  "), "{text}");
    assert!(!text.contains("WARNING  "), "{text}");
    assert!(!text.contains("(unspecified)"), "{text}");
    assert!(
        text.contains(
            "COVERAGE BY MODULE\n  bug: 12 edges checked, 2 requirements unverified (precision 1, range_min 1)\n  ok: 12 edges checked, 0 requirements unverified\n"
        ),
        "{text}"
    );
    assert!(text.contains("Pass --warnings to list them."), "{text}");
    assert!(text.ends_with("RESULT: 3 errors, 0 warnings, 2 unverified. Exit code 1.\n"), "{text}");

    let (text, code) = run_fixture(fixture, &db, &["--format", "text", "--warnings"]).unwrap();
    assert_eq!(code, Some(1));
    assert_eq!(text.matches("\nUNVERIFIED  ").count(), 2, "{text}");
    assert!(
        text.contains("UNVERIFIED  bug.py:26 \u{2192} models.py:7\n       no precision guarantee for the value fee_of passes to StockItem.fee\n"),
        "{text}"
    );
    assert!(text.ends_with("RESULT: 3 errors, 0 warnings, 2 unverified. Exit code 1.\n"), "{text}");
}

#[test]
fn warnings_and_no_warnings_conflict() {
    let output = Command::new(cli_binary())
        .args(["contracts", "check", "test_fixtures/r6_alternatives/", "--format", "text", "--warnings", "--no-warnings"])
        .output()
        .expect("failed to run crosscheck-contracts");
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot be used with"));
}

/// `--no-warnings` hides the transitive fixture's dependent-bound warning,
/// which the default report prints (UC-2, UC-4).
#[test]
fn no_warnings_hides_the_dependent_bound_warning() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db = tmp.path().join("c.sqlite");
    let Some((text, _)) = run_fixture("test_fixtures/transitive/", &db, &["--format", "text"]) else { return };
    assert!(text.contains("\nWARNING  "), "{text}");
    let (text, code) = run_fixture("test_fixtures/transitive/", &db, &["--format", "text", "--no-warnings"]).unwrap();
    assert_eq!(code, Some(1));
    assert!(!text.contains("WARNING  "), "{text}");
    assert!(text.ends_with("RESULT: 1 error, 1 warning, 0 unverified. Exit code 1.\n"), "{text}");
}
