//! E2E tests for `contracts check --witness-tests DIR` (WX-1 to WX-8): runs
//! the built CLI and the Lean checker on the witness fixtures and checks the
//! modules it writes. `scripts/witness_tests.py` runs them under pytest.
//!
//! Skipped (with an explanation) if the Lean checker binary has not been built.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn lean_checker_binary() -> Option<PathBuf> {
    let checker = PathBuf::from("prover/.lake/build/bin/contract-graph-checker");
    if checker.is_file() {
        Some(checker)
    } else {
        eprintln!(
            "SKIP e2e_witness_tests: Lean checker binary not found at {} \
             (run `cd prover && lake build` first)",
            checker.display()
        );
        None
    }
}

fn check(fixture: &str, checker: &Path, witness_dir: Option<&Path>) -> Output {
    let db = tempfile::TempDir::new().unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_crosscheck-contracts"));
    command
        .args(["contracts", "check"])
        .arg(format!("test_fixtures/{fixture}/"))
        .arg("--lean-checker")
        .arg(checker)
        .arg("--output-db")
        .arg(db.path().join("contracts.sqlite"));
    if let Some(dir) = witness_dir {
        command.arg("--witness-tests").arg(dir);
    }
    command.output().expect("failed to run crosscheck-contracts")
}

fn files(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn writes_one_module_per_error_on_a_django_or_pydantic_field() {
    let Some(checker) = lean_checker_binary() else {
        return;
    };
    let dir = tempfile::TempDir::new().unwrap();
    let out = check("witness_tests", &checker, Some(dir.path()));
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("wrote 15 witness test modules to")
            && stderr.contains("; 1 witness error not yet reached (1 target not enforced at run time)"),
        "{stderr}"
    );

    let expected: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string("test_fixtures/witness_tests/expected.json").unwrap()).unwrap();
    let errors: Vec<&serde_json::Value> = expected["errors"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["target"] != "Draft.title")
        .collect();
    let names = files(dir.path());
    assert_eq!(names.len(), errors.len() + 1, "{names:?}");
    assert_eq!(names[0], "_cgv_witness.py");
    for (n, (name, error)) in names[1..].iter().zip(&errors).enumerate() {
        let target = error["target"].as_str().unwrap();
        let slug = target.to_lowercase().replace('.', "_");
        assert_eq!(name, &format!("test_cgv_witness_{:03}_{slug}.py", n + 1));
        let text = std::fs::read_to_string(dir.path().join(name)).unwrap();
        let counterexample = error["counterexample"].as_str().unwrap();
        assert!(text.contains(&format!("\nCOUNTEREXAMPLE = {counterexample}\n")), "{name}:\n{text}");
        assert!(text.contains(&format!("# Error: {}", error["path"].as_str().unwrap())), "{text}");
        assert!(text.contains("def test_target_accepts_counterexample():"), "{text}");
        let library = if target.starts_with("Account.") { "django" } else { "pydantic" };
        let module = if library == "django" { "models" } else { "records" };
        let (class, field) = target.split_once('.').unwrap();
        assert!(
            text.contains(&format!("TARGET = (\"{library}\", \"{module}\", \"{class}\", \"{field}\")")),
            "{text}"
        );
        let fractional = counterexample.starts_with("Decimal(") && counterexample.contains('.');
        assert!(text.contains(&format!("FRACTIONAL = {}", if fractional { "True" } else { "False" })));
    }
    let helper = std::fs::read_to_string(dir.path().join("_cgv_witness.py")).unwrap();
    assert!(helper.contains("\nDJANGO_MODULES = [\"models\"]\n"), "{helper}");
    let root = std::path::absolute("test_fixtures/witness_tests/").unwrap();
    assert!(helper.contains(&format!("\nAPP_ROOT = {}\n", serde_json::to_string(&root.to_string_lossy()).unwrap())));
}

#[test]
fn the_flag_leaves_stdout_and_the_exit_code_unchanged() {
    let Some(checker) = lean_checker_binary() else {
        return;
    };
    let dir = tempfile::TempDir::new().unwrap();
    let with = check("witness_tests", &checker, Some(dir.path()));
    let without = check("witness_tests", &checker, None);
    assert_eq!(with.status.code(), without.status.code());
    assert_eq!(with.stdout, without.stdout);
}

#[test]
fn a_run_with_no_error_writes_no_module_and_clears_earlier_ones() {
    let Some(checker) = lean_checker_binary() else {
        return;
    };
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("test_mine.py"), "def test_mine(): pass\n").unwrap();
    check("witness_tests", &checker, Some(dir.path()));
    assert_eq!(files(dir.path()).len(), 17);

    let out = check("witness_tests_clean", &checker, Some(dir.path()));
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stderr).contains("wrote 0 witness test modules to"));
    assert_eq!(files(dir.path()), ["test_mine.py"]);
}

#[test]
fn a_directory_that_cannot_be_written_exits_2() {
    let Some(checker) = lean_checker_binary() else {
        return;
    };
    let dir = tempfile::TempDir::new().unwrap();
    let file = dir.path().join("not-a-dir");
    std::fs::write(&file, "").unwrap();
    let out = check("witness_tests", &checker, Some(&file));
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("cannot write the witness tests to"));
}
