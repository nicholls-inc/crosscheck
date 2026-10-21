//! E2E tests for error counterexamples (WT-1 to WT-9): runs the built CLI
//! and the Lean checker on fixtures with a length, precision, range,
//! range_min, choices and nullability error, and checks that each error
//! carries the counterexample its source guarantee implies, in the JSON and
//! in the text report.
//!
//! Skipped (with an explanation) if the Lean checker binary has not been built.

use std::path::PathBuf;
use std::process::Command;

const FIXTURES: [&str; 5] = [
    "plain_python",
    "bug1",
    "r3_float_bounds",
    "r3_choices",
    "nullable",
];

fn lean_checker_binary() -> Option<PathBuf> {
    let checker = PathBuf::from("prover/.lake/build/bin/contract-graph-checker");
    if checker.is_file() {
        Some(checker)
    } else {
        eprintln!(
            "SKIP e2e_counterexample: Lean checker binary not found at {} \
             (run `cd prover && lake build` first)",
            checker.display()
        );
        None
    }
}

fn check(fixture: &str, checker: &PathBuf, format: &str) -> String {
    let tmp = tempfile::TempDir::new().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_crosscheck-contracts"))
        .args(["contracts", "check"])
        .arg(format!("test_fixtures/{fixture}/"))
        .arg("--lean-checker")
        .arg(checker)
        .args(["--format", format])
        .arg("--output-db")
        .arg(tmp.path().join("contracts.sqlite"))
        .output()
        .expect("failed to run crosscheck-contracts");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn bound(guarantee: &str, prefix: &str) -> String {
    guarantee
        .strip_prefix(prefix)
        .unwrap_or_else(|| panic!("{guarantee:?} does not start with {prefix:?}"))
        .to_string()
}

fn choice_list(guarantee: &str) -> Vec<String> {
    let inner = guarantee
        .strip_prefix("choices in [")
        .and_then(|rest| rest.strip_suffix(']'))
        .unwrap_or_else(|| panic!("{guarantee:?} is not a choices guarantee"));
    inner.split(", ").map(str::to_string).collect()
}

/// The counterexample WT-2 to WT-7 give for an error, derived from the
/// source and target guarantee texts.
fn expected_counterexample(source: &str, target: &str) -> String {
    if source == "nullable" {
        return "None".to_string();
    }
    if let Some(s) = source.strip_prefix("length ≤ ") {
        return format!("\"x\" * {s}");
    }
    if source.starts_with("precision ≤ ") {
        let s: usize = bound(source, "precision ≤ ").parse().unwrap();
        return format!("Decimal(\"0.{}1\")", "0".repeat(s - 1));
    }
    if source.starts_with("range ≤ ") || source.starts_with("range ≥ ") {
        let s = &source["range ≤ ".len()..];
        return if s.contains('.') {
            format!("Decimal(\"{s}\")")
        } else {
            s.to_string()
        };
    }
    if source.starts_with("choices in [") {
        let target_choices = choice_list(target);
        let first = choice_list(source)
            .into_iter()
            .find(|c| !target_choices.contains(c))
            .expect("a choices error has a source choice outside the target");
        return format!("\"{first}\"");
    }
    panic!("unknown guarantee kind: {source:?}");
}

fn errors_of(json: &str) -> Vec<serde_json::Value> {
    let value: serde_json::Value = serde_json::from_str(json).expect("checker output is JSON");
    value["results"]
        .as_array()
        .expect("results array")
        .iter()
        .filter(|r| r["severity"] == "error")
        .cloned()
        .collect()
}

#[test]
fn json_errors_carry_the_counterexample_of_their_source_guarantee() {
    let Some(checker) = lean_checker_binary() else { return };
    for fixture in FIXTURES {
        let json = check(fixture, &checker, "json");
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        for result in value["results"].as_array().unwrap() {
            if result["severity"] != "error" {
                assert!(
                    result.get("counterexample").is_none(),
                    "{fixture}: a non-error carries a counterexample: {result}"
                );
            }
        }
        let errors = errors_of(&json);
        assert!(!errors.is_empty(), "{fixture}: expected errors");
        for error in errors {
            let source = error["source_guarantee"].as_str().unwrap();
            let target = error["target_requirement"].as_str().unwrap();
            let want = expected_counterexample(source, target);
            assert_eq!(
                error["counterexample"].as_str(),
                Some(want.as_str()),
                "{fixture}: {source} vs {target}"
            );
        }
    }
}

#[test]
fn text_error_blocks_end_with_the_counterexample_line() {
    let Some(checker) = lean_checker_binary() else { return };
    for fixture in FIXTURES {
        let errors = errors_of(&check(fixture, &checker, "json"));
        let text = check(fixture, &checker, "text");
        let blocks: Vec<Vec<&str>> = text
            .split("\n\n")
            .filter(|b| b.starts_with("ERROR"))
            .map(|b| b.lines().collect())
            .collect();
        assert_eq!(blocks.len(), errors.len(), "{fixture}: error blocks in\n{text}");
        for error in errors {
            let source = error["source_guarantee"].as_str().unwrap();
            let target = error["target_requirement"].as_str().unwrap();
            let want = format!(
                "       counterexample: {}",
                expected_counterexample(source, target)
            );
            let guarantee_line = format!(
                "       {} guarantees {source}",
                error["source"]["name"].as_str().unwrap()
            );
            let requirement_line = format!(
                "       {} requires {target}",
                error["target"]["name"].as_str().unwrap()
            );
            let block = blocks
                .iter()
                .find(|b| b.contains(&guarantee_line.as_str()) && b.contains(&requirement_line.as_str()))
                .unwrap_or_else(|| panic!("{fixture}: no block for {source} vs {target}"));
            assert_eq!(block.last().copied(), Some(want.as_str()), "{fixture}: {source}");
            assert!(
                block[block.len() - 2].starts_with("       Suggestion:"),
                "{fixture}: the counterexample follows the suggestion"
            );
        }
    }
}
