//! E2E test for `contracts check --format text`: runs the built CLI binary
//! end-to-end (extractor + Lean checker) on the transitive fixture and
//! checks the human-readable report shape described in
//! `docs/design/system-design.md` section 5.3.
//!
//! Skipped (with an explanation) if either the CLI binary or the Lean
//! checker binary has not been built yet.

use std::path::{Path, PathBuf};
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
    assert!(stdout.contains("PATHS CHECKED:"), "got:\n{stdout}");

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
        stdout.contains("RESULT: 1 error, 1 warning. Exit code 1."),
        "got:\n{stdout}"
    );
    assert_eq!(output.status.code(), Some(1));

    // Sanity check that the fixture path is a real, existing directory
    // relative to the crate root the test runner uses.
    assert!(Path::new("test_fixtures/transitive").is_dir());
}
