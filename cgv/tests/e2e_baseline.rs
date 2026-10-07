//! `contracts check --baseline` / `--write-baseline` end to end (spec:
//! `intent/2026-10-07-cgv-baseline-mode-spec.md`). A stand-in checker (a
//! shell script that prints a fixed JSON result) replaces the Lean binary.

use std::path::PathBuf;
use std::process::{Command, Output};

use serde_json::{json, Value};
use tempfile::TempDir;

struct Fixture {
    tmp: TempDir,
}

impl Fixture {
    fn new(source: &str) -> Self {
        let fixture = Fixture { tmp: TempDir::new().unwrap() };
        std::fs::create_dir_all(fixture.app()).unwrap();
        fixture.set_source(source);
        fixture
    }

    fn path(&self, name: &str) -> PathBuf {
        self.tmp.path().join(name)
    }

    fn app(&self) -> PathBuf {
        self.path("app")
    }

    fn set_source(&self, source: &str) {
        std::fs::write(self.app().join("m.py"), source).unwrap();
    }

    /// Make the stand-in checker print `results` and exit `code`; it also
    /// creates the file `ran`.
    fn set_checker(&self, results: &[Value], code: i32) {
        use std::os::unix::fs::PermissionsExt;
        let output = json!({
            "summary": {"contracts_checked": 3, "edges_checked": 2, "paths_checked": 2},
            "results": results,
            "exit_code": code,
        });
        std::fs::write(self.path("result.json"), output.to_string()).unwrap();
        let script = self.path("checker.sh");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\ntouch '{}'\ncat '{}'\nexit {code}\n",
                self.path("ran").display(),
                self.path("result.json").display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn run(&self, extra: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_crosscheck-contracts"))
            .args(["contracts", "check"])
            .arg(self.app())
            .arg("--lean-checker")
            .arg(self.path("checker.sh"))
            .arg("--output-db")
            .arg(self.path("c.sqlite"))
            .args(extra)
            .current_dir(self.tmp.path())
            .output()
            .expect("run crosscheck-contracts")
    }
}

const SOURCE: &str = "def f(y):\n    x = round(y, 4)\n    z = round(y, 4)\n    return x\n";

/// A result of `f` writing `M.<field>` at line `line` of `m.py`.
fn result(severity: &str, guarantee: &str, field: &str, line: i64) -> Value {
    json!({
        "status": "inconsistent", "severity": severity,
        "source": {"file": "m.py", "line": 1, "name": "f"},
        "target": {"file": "models.py", "line": 3, "name": format!("M.{field}")},
        "path": ["f", format!("M.{field}")], "witness": ["f", format!("M.{field}")],
        "hop": ["f", format!("M.{field}")],
        "site": {"file": "m.py", "line": line},
        "guarantee_at": {"file": "m.py", "line": line},
        "source_guarantee": guarantee, "target_requirement": "precision ≤ 2",
        "verification_level": "EXTRACTED", "suggestion": "s",
    })
}

fn error(field: &str, line: i64) -> Value {
    result("error", "precision ≤ 4", field, line)
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn json_of(out: &Output) -> Value {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {}", stdout(out)))
}

fn write_baseline(f: &Fixture, results: &[Value], code: i32) -> String {
    f.set_checker(results, code);
    let path = f.path("baseline.json");
    let out = f.run(&["--write-baseline", path.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(code), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(path.is_file());
    path.to_str().unwrap().to_string()
}

#[test]
fn identical_run_exits_0_and_counts_every_finding_as_existing() {
    let f = Fixture::new(SOURCE);
    let results = [
        error("x", 2),
        result("warning", "precision (dependent)", "y", 3),
        result("warning", "precision (unspecified)", "z", 3),
    ];
    f.set_checker(&results, 1);
    let checker_json = std::fs::read(f.path("result.json")).unwrap();
    let path = f.path("baseline.json");
    let out = f.run(&["--write-baseline", path.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(out.stdout, checker_json, "writing a baseline changes the output");
    let file: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(file["format"], "cgv-baseline/1");
    assert_eq!(
        file["findings"][0],
        json!({
            "class": "error", "source": "f", "target": "M.x", "hop": ["f", "M.x"],
            "source_guarantee": "precision ≤ 4", "target_requirement": "precision ≤ 2",
            "site_file": "m.py", "site_text": "x = round(y, 4)",
        })
    );
    assert_eq!(file["findings"].as_array().unwrap().len(), 3);

    let out = f.run(&["--baseline", path.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    let out = json_of(&out);
    assert_eq!(out["results"], json!([]));
    assert_eq!(out["exit_code"], 0);
    assert_eq!(out["summary"]["contracts_checked"], 3);
    assert_eq!(
        out["baseline"],
        json!({
            "checker_exit_code": 1,
            "new": {"errors": 0, "warnings": 0, "unverified": 0},
            "existing": {"errors": 1, "warnings": 1, "unverified": 1},
            "fixed": [],
        })
    );

    let out = f.run(&["--baseline", path.to_str().unwrap(), "--format", "text"]);
    assert_eq!(out.status.code(), Some(0));
    let text = stdout(&out);
    assert!(!text.contains("ERROR  ") && !text.contains("WARNING  "), "{text}");
    assert!(!text.contains("FIXED:"), "{text}");
    assert!(text.contains("  m: 0 edges checked, 1 requirement unverified (precision 1)\n"), "{text}");
    assert!(
        text.ends_with(
            "BASELINE: 1 error, 1 warning, 1 unverified in the baseline, not shown.\n\
             RESULT: 0 new errors, 0 new warnings, 0 new unverified. Exit code 0.\n"
        ),
        "{text}"
    );
}

#[test]
fn an_error_not_in_the_baseline_exits_1_and_is_shown() {
    let f = Fixture::new(SOURCE);
    let path = write_baseline(&f, &[error("x", 2)], 1);
    f.set_checker(&[error("x", 2), error("z", 3)], 1);

    let out = f.run(&["--baseline", &path]);
    assert_eq!(out.status.code(), Some(1));
    let out = json_of(&out);
    assert_eq!(out["results"], json!([error("z", 3)]));
    assert_eq!(out["exit_code"], 1);
    assert_eq!(out["baseline"]["new"], json!({"errors": 1, "warnings": 0, "unverified": 0}));
    assert_eq!(out["baseline"]["existing"], json!({"errors": 1, "warnings": 0, "unverified": 0}));

    let out = f.run(&["--baseline", &path, "--format", "text"]);
    assert_eq!(out.status.code(), Some(1));
    let text = stdout(&out);
    assert_eq!(text.matches("\nERROR  ").count(), 1, "{text}");
    assert!(text.contains("       M.z requires precision ≤ 2\n"), "{text}");
    assert!(!text.contains("M.x requires"), "{text}");
    assert!(
        text.ends_with(
            "BASELINE: 1 error, 0 warnings, 0 unverified in the baseline, not shown.\n\
             RESULT: 1 new error, 0 new warnings, 0 new unverified. Exit code 1.\n"
        ),
        "{text}"
    );
}

#[test]
fn a_second_error_with_a_baseline_key_makes_both_new() {
    let f = Fixture::new(SOURCE);
    let path = write_baseline(&f, &[error("x", 2)], 1);
    f.set_checker(&[error("x", 2), error("x", 2)], 1);
    let out = f.run(&["--baseline", &path]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(json_of(&out)["results"], json!([error("x", 2), error("x", 2)]));
}

#[test]
fn an_error_moved_down_by_an_edit_above_stays_existing() {
    let f = Fixture::new(SOURCE);
    let path = write_baseline(&f, &[error("x", 2)], 1);
    f.set_source("import os\n\n\ndef f(y):\n    x =  round(y, 4)\n    return x\n");
    f.set_checker(&[error("x", 5)], 1);
    let out = f.run(&["--baseline", &path]);
    assert_eq!(out.status.code(), Some(0), "{}", stdout(&out));
    let out = json_of(&out);
    assert_eq!(out["results"], json!([]));
    assert_eq!(out["baseline"]["existing"]["errors"], 1);
}

#[test]
fn an_edit_to_the_site_line_makes_the_error_new() {
    let f = Fixture::new(SOURCE);
    let path = write_baseline(&f, &[error("x", 2)], 1);
    f.set_source("def f(y):\n    x = round(y, 5)\n    return x\n");
    let out = f.run(&["--baseline", &path]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(json_of(&out)["results"], json!([error("x", 2)]));
}

#[test]
fn a_baseline_finding_the_run_no_longer_reports_is_fixed() {
    let f = Fixture::new(SOURCE);
    let path = write_baseline(&f, &[error("x", 2), error("z", 3)], 1);
    f.set_checker(&[error("x", 2)], 1);

    let out = f.run(&["--baseline", &path]);
    assert_eq!(out.status.code(), Some(0));
    let out = json_of(&out);
    assert_eq!(out["results"], json!([]));
    assert_eq!(
        out["baseline"]["fixed"],
        json!([{
            "class": "error", "source": "f", "target": "M.z", "hop": ["f", "M.z"],
            "source_guarantee": "precision ≤ 4", "target_requirement": "precision ≤ 2",
            "site_file": "m.py", "site_text": "z = round(y, 4)",
        }])
    );

    let out = f.run(&["--baseline", &path, "--format", "text"]);
    assert_eq!(out.status.code(), Some(0));
    let text = stdout(&out);
    assert!(
        text.ends_with(
            "BASELINE: 1 error, 0 warnings, 0 unverified in the baseline, not shown.\n\
             FIXED: 1 finding in the baseline not reported by this run:\n  \
             error f \u{2192} M.z at m.py: z = round(y, 4)\n\
             RESULT: 0 new errors, 0 new warnings, 0 new unverified. Exit code 0.\n"
        ),
        "{text}"
    );
}

#[test]
fn baseline_and_evidence_record_conflict() {
    let f = Fixture::new(SOURCE);
    let path = write_baseline(&f, &[error("x", 2)], 1);
    std::fs::remove_file(f.path("ran")).unwrap();
    let out = f.run(&["--baseline", &path, "--evidence-record", f.path("r.json").to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("cannot be used with"));
    assert!(!f.path("ran").exists());
}

#[test]
fn a_bad_baseline_exits_2_before_the_checker_runs() {
    let f = Fixture::new(SOURCE);
    let good = std::fs::read_to_string(write_baseline(&f, &[error("x", 2)], 1)).unwrap();
    let bad = f.path("bad.json");
    for (contents, why) in [
        (None, "missing"),
        (Some("not json".to_string()), "not JSON"),
        (Some(good.replace("cgv-baseline/1", "cgv-baseline/0")), "wrong format"),
        (Some(good.replace("\"site_text\"", "\"site_txt\"")), "missing field"),
    ] {
        match &contents {
            Some(c) => std::fs::write(&bad, c).unwrap(),
            None => {
                let _ = std::fs::remove_file(&bad);
            }
        }
        let _ = std::fs::remove_file(f.path("ran"));
        let out = f.run(&["--baseline", bad.to_str().unwrap()]);
        assert_eq!(out.status.code(), Some(2), "{why}");
        assert!(out.stdout.is_empty(), "{why}");
        assert!(String::from_utf8_lossy(&out.stderr).contains("bad.json"), "{why}");
        assert!(!f.path("ran").exists(), "{why}: the checker ran");
    }
}

#[test]
fn an_incomplete_run_exits_2_and_writes_no_baseline() {
    let f = Fixture::new(SOURCE);
    let path = write_baseline(&f, &[error("x", 2)], 1);
    let stale = f.path("stale.json");
    std::fs::write(&stale, "{}").unwrap();
    let incomplete = json!({
        "status": "incomplete", "severity": "error",
        "source": {"file": "", "line": 0, "name": ""}, "target": {"file": "", "line": 0, "name": ""},
        "path": [], "hop": [], "site": {"file": "", "line": 0},
        "source_guarantee": "", "target_requirement": "", "verification_level": "",
        "suggestion": "State budget exceeded.",
    });
    for code in [2, 0] {
        f.set_checker(&[error("x", 2), incomplete.clone()], code);
        std::fs::write(&stale, "{}").unwrap();
        let out = f.run(&["--baseline", &path, "--write-baseline", stale.to_str().unwrap()]);
        assert_eq!(out.status.code(), Some(2), "checker exit {code}");
        assert!(!stale.exists(), "checker exit {code}: a baseline was left");
        let out = json_of(&out);
        assert_eq!(out["results"], json!([incomplete]));
        assert_eq!(out["exit_code"], 2);
    }
    let out = f.run(&["--baseline", &path, "--format", "text"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(stdout(&out).contains("INCOMPLETE  State budget exceeded."));
    let out = f.run(&["--write-baseline", stale.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    assert!(!stale.exists());
}

#[test]
fn a_failing_checker_writes_no_baseline() {
    let f = Fixture::new(SOURCE);
    let path = f.path("b.json");
    std::fs::write(&path, "{}").unwrap();
    f.set_checker(&[error("x", 2)], 3);
    let out = f.run(&["--write-baseline", path.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(3));
    assert!(!path.exists());
}

#[test]
fn an_unreadable_site_exits_2_naming_the_file() {
    let f = Fixture::new(SOURCE);
    let mut gone = error("x", 2);
    gone["site"]["file"] = json!("gone.py");
    f.set_checker(&[gone], 1);
    let out = f.run(&["--write-baseline", f.path("b.json").to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("gone.py"));
    assert!(!f.path("b.json").exists());
}

#[test]
fn checker_exit_1_without_errors_exits_1() {
    let f = Fixture::new(SOURCE);
    let path = write_baseline(&f, &[], 0);
    f.set_checker(&[], 1);
    let out = f.run(&["--baseline", &path]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(json_of(&out)["baseline"]["checker_exit_code"], 1);
}

#[test]
fn a_single_file_app_reads_sites_beside_it() {
    let f = Fixture::new(SOURCE);
    f.set_checker(&[error("x", 2)], 1);
    let path = f.path("b.json");
    let out = Command::new(env!("CARGO_BIN_EXE_crosscheck-contracts"))
        .args(["contracts", "check"])
        .arg(f.app().join("m.py"))
        .arg("--lean-checker")
        .arg(f.path("checker.sh"))
        .arg("--output-db")
        .arg(f.path("c.sqlite"))
        .arg("--write-baseline")
        .arg(&path)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1), "{}", String::from_utf8_lossy(&out.stderr));
    let file: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(file["findings"][0]["site_text"], "x = round(y, 4)");
}

#[test]
fn a_json_exit_code_the_process_contradicts_is_not_a_completed_run() {
    let f = Fixture::new(SOURCE);
    let path = write_baseline(&f, &[], 0);
    f.set_checker(&[], 0);
    let script = std::fs::read_to_string(f.path("checker.sh")).unwrap();
    std::fs::write(f.path("checker.sh"), script.replace("exit 0", "exit 1")).unwrap();
    let written = f.path("w.json");
    let out = f.run(&["--baseline", &path, "--write-baseline", written.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(2));
    assert!(!written.exists());
}
