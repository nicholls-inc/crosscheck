//! `--evidence-record`: the record a run that exits 0 leaves, and the runs
//! that must leave none. A stand-in checker (a shell script) replaces the Lean
//! binary, and each project is a scratch git repository.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{json, Value};
use tempfile::TempDir;

const TOOLCHAIN: &str = include_str!("../prover/lean-toolchain");
const BUILD_COMMIT: &str = env!("CGV_BUILD_COMMIT");

const CLEAN_JSON: &str = r#"{"summary": {"contracts_checked": 1, "edges_checked": 0, "paths_checked": 0}, "results": [], "exit_code": 0}"#;
const CLEAN_BODY: &str = "printf %s '{\"summary\": {\"contracts_checked\": 1, \"edges_checked\": 0, \"paths_checked\": 0}, \"results\": [], \"exit_code\": 0}'\nexit 0";
/// `sha256` of the script that `fake_checker(_, CLEAN_BODY)` writes.
const CLEAN_CHECKER_SHA256: &str = "b3ee744f36e228dd14981784898a4978ed0a3a8e82a2d9a94997a9832c578d26";

fn cli() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_crosscheck-contracts"))
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false"])
        .args(args)
        .output()
        .expect("run git");
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

fn fake_checker(dir: &Path, body: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("checker.sh");
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path.canonicalize().unwrap()
}

/// A scratch layout: `repo/` is a git repository with one commit holding
/// `repo/app/m.py`. The checker, the record and the database live outside it.
struct Scratch {
    tmp: TempDir,
}

impl Scratch {
    fn new(source: &str) -> Scratch {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(repo.join("app")).unwrap();
        std::fs::write(repo.join("app/m.py"), source).unwrap();
        git(&repo, &["init", "-q"]);
        git(&repo, &["add", "."]);
        git(&repo, &["commit", "-q", "-m", "init"]);
        Scratch { tmp }
    }

    fn repo(&self) -> PathBuf {
        self.tmp.path().join("repo")
    }

    fn record(&self) -> PathBuf {
        self.tmp.path().join("record.json")
    }

    fn checker(&self, body: &str) -> PathBuf {
        fake_checker(self.tmp.path(), body)
    }

    fn head(&self) -> String {
        git(&self.repo(), &["rev-parse", "HEAD"])
    }

    fn run(&self, checked: &Path, checker: &Path, extra: &[&str]) -> Output {
        Command::new(cli())
            .current_dir(self.repo())
            .args(["contracts", "check"])
            .arg(checked)
            .arg("--lean-checker")
            .arg(checker)
            .arg("--output-db")
            .arg(self.tmp.path().join("c.sqlite"))
            .arg("--evidence-record")
            .arg(self.record())
            .args(extra)
            .output()
            .expect("run crosscheck-contracts")
    }

    fn read_record(&self) -> Value {
        serde_json::from_slice(&std::fs::read(self.record()).expect("record written")).unwrap()
    }
}

fn components(record: &Value) -> Vec<(String, String)> {
    record["claims"][0]["trusted_base"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| (c["component"].as_str().unwrap().to_string(), c["version"].as_str().unwrap().to_string()))
        .collect()
}

fn statement(warnings: &str) -> String {
    format!(
        "At every hop of every checked data path from a function to a model field, the source's guarantees imply the target's requirements, for the contracts the CGV extractor derived from the checked path at that commit. The checker reported {warnings}. A warning marks a requirement CGV could not show, which the theorem does not cover."
    )
}

#[test]
fn exit_0_writes_the_record() {
    let s = Scratch::new("def f(x):\n    return x\n");
    let checker = s.checker(CLEAN_BODY);
    let out = s.run(&s.repo().join("app"), &checker, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(out.stdout, CLEAN_JSON.as_bytes());

    let toolchain = TOOLCHAIN.trim();
    assert_eq!(toolchain, "leanprover/lean4:v4.28.0");
    let command = format!(
        "crosscheck-contracts contracts check app --django-version 4.2 --lean-checker {}",
        checker.display()
    );
    let expected = json!({
        "format": "evidence-record/1",
        "commit": s.head(),
        "claims": [{
            "id": "cgv-data-paths",
            "statement": statement("0 warnings"),
            "requirement": null,
            "strength": "proved",
            "basis": {"theorems": ["ContractGraph.runChecker_sound_all"]},
            "trusted_base": [
                {"component": "Lean 4 kernel", "version": toolchain},
                {"component": "Lean axioms propext, Classical.choice and Quot.sound", "version": toolchain},
                {"component": "Lean compiler and runtime that built contract-graph-checker", "version": toolchain},
                {"component": "contract-graph-checker binary", "version": format!("sha256:{CLEAN_CHECKER_SHA256}")},
                {"component": "CGV Rust extractor and command line", "version": BUILD_COMMIT},
                {"component": "CGV Translation.lean", "version": BUILD_COMMIT},
                {"component": "CGV BehaviorModel.lean", "version": BUILD_COMMIT},
            ],
            "rerun": {"command": command, "exit_code": 0}
        }]
    });
    let record = s.read_record();
    assert_eq!(record, expected);
    assert_eq!(s.head().len(), 40);
    let build = BUILD_COMMIT.strip_suffix("-dirty").unwrap_or(BUILD_COMMIT);
    assert!(build == "unknown" || (build.len() == 40 && build.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'))));
}

#[test]
fn a_warning_is_counted_in_the_statement() {
    let s = Scratch::new("def f(x):\n    return x\n");
    let warning = r#"{"status":"warning","severity":"warning","source":{"file":"m.py","line":1,"name":"f"},"target":{"file":"m.py","line":2,"name":"T.x"},"path":["f","T.x"],"hop":["f","T.x"],"source_guarantee":"g","target_requirement":"r","verification_level":"EXTRACTED","suggestion":"s"}"#;
    let body = format!(
        "printf %s '{{\"summary\": {{\"contracts_checked\": 1, \"edges_checked\": 1, \"paths_checked\": 1}}, \"results\": [{warning}], \"exit_code\": 0}}'\nexit 0"
    );
    let out = s.run(&s.repo().join("app"), &s.checker(&body), &[]);
    assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(s.read_record()["claims"][0]["statement"], json!(statement("1 warning")));
}

#[test]
fn the_rerun_command_exits_0_from_the_work_tree_root() {
    let s = Scratch::new("def f(x):\n    return x\n");
    std::fs::write(s.repo().join("o.toml"), "").unwrap();
    git(&s.repo(), &["add", "."]);
    git(&s.repo(), &["commit", "-q", "-m", "overrides"]);
    let checker = s.checker(CLEAN_BODY);
    let out = s.run(
        &s.repo().join("app"),
        &checker,
        &["--overrides", s.repo().join("o.toml").to_str().unwrap(), "--exclude", "*.txt", "--allow-parse-errors", "--max-states", "5", "--max-states-per-edge", "2"],
    );
    assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
    let command = s.read_record()["claims"][0]["rerun"]["command"].as_str().unwrap().to_string();
    assert_eq!(
        command,
        format!(
            "crosscheck-contracts contracts check app --django-version 4.2 --overrides o.toml --exclude '*.txt' --allow-parse-errors --max-states 5 --max-states-per-edge 2 --lean-checker {}",
            checker.display()
        )
    );

    let path = format!("{}:{}", cli().parent().unwrap().display(), std::env::var("PATH").unwrap());
    let rerun = Command::new("sh")
        .args(["-c", &command])
        .current_dir(s.repo())
        .env("PATH", path)
        .output()
        .unwrap();
    assert_eq!(rerun.status.code(), Some(0), "{}", String::from_utf8_lossy(&rerun.stderr));
    assert_eq!(rerun.stdout, CLEAN_JSON.as_bytes());
}

#[test]
fn an_argument_with_a_space_and_a_quote_survives_the_rerun_command() {
    let s = Scratch::new("def f(x):\n    return x\n");
    let glob = "a b'c";
    let out = s.run(&s.repo().join("app"), &s.checker(CLEAN_BODY), &["--exclude", glob]);
    assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
    let command = s.read_record()["claims"][0]["rerun"]["command"].as_str().unwrap().to_string();
    assert!(command.contains(r"--exclude 'a b'\''c' "), "{command}");

    let bin = s.tmp.path().join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let args_file = s.tmp.path().join("args.txt");
    let shim = bin.join("crosscheck-contracts");
    std::fs::write(&shim, format!("#!/bin/sh\nfor a in \"$@\"; do printf '%s\\n' \"$a\"; done > '{}'\n", args_file.display())).unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let status = Command::new("sh")
        .args(["-c", &command])
        .current_dir(s.repo())
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .status()
        .unwrap();
    assert!(status.success());
    let args = std::fs::read_to_string(&args_file).unwrap();
    let lines: Vec<&str> = args.lines().collect();
    assert_eq!(&lines[..7], ["contracts", "check", "app", "--django-version", "4.2", "--exclude", glob]);
}

#[test]
fn exit_1_removes_a_record_from_an_earlier_run() {
    let s = Scratch::new("def f(x):\n    return x\n");
    std::fs::write(s.record(), "stale").unwrap();
    let body = "printf %s '{\"summary\": {\"contracts_checked\": 1, \"edges_checked\": 0, \"paths_checked\": 0}, \"results\": [], \"exit_code\": 1}'\nexit 1";
    let out = s.run(&s.repo().join("app"), &s.checker(body), &[]);
    assert_eq!(out.status.code(), Some(1));
    assert!(!s.record().exists());
}

#[test]
fn an_incomplete_run_removes_a_record_from_an_earlier_run() {
    for (name, body) in [
        ("exit 2 with JSON", "printf %s '{\"summary\": {\"contracts_checked\": 1, \"edges_checked\": 0, \"paths_checked\": 0}, \"results\": [], \"exit_code\": 2}'\nexit 2"),
        ("output that is not JSON", "printf %s 'not json'\nexit 0"),
        ("a crash with no output", "exit 139"),
    ] {
        let s = Scratch::new("def f(x):\n    return x\n");
        std::fs::write(s.record(), "stale").unwrap();
        let out = s.run(&s.repo().join("app"), &s.checker(body), &[]);
        assert_ne!(out.status.code(), Some(0), "{name}");
        assert!(!s.record().exists(), "{name}: a record survived");
    }
}

#[test]
fn a_record_inside_the_checked_path_does_not_make_the_next_run_dirty() {
    let s = Scratch::new("def f(x):\n    return x\n");
    let inside = s.repo().join("app/record.json");
    std::fs::write(&inside, "stale").unwrap();
    let out = Command::new(cli())
        .args(["contracts", "check"])
        .arg(s.repo().join("app"))
        .arg("--lean-checker")
        .arg(s.checker(CLEAN_BODY))
        .arg("--output-db")
        .arg(s.tmp.path().join("c.sqlite"))
        .arg("--evidence-record")
        .arg(&inside)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
    let record: Value = serde_json::from_slice(&std::fs::read(&inside).unwrap()).unwrap();
    assert_eq!(record["format"], "evidence-record/1");
}

/// A refused run exits 2 with a message on stderr only, never runs the
/// checker, and leaves no record (a stale one is removed).
fn assert_refused(s: &Scratch, checked: &Path, extra: &[&str], stderr_has: &str) {
    let marker = s.tmp.path().join("ran");
    let checker = s.checker(&format!("touch '{}'\n{CLEAN_BODY}", marker.display()));
    std::fs::write(s.record(), "stale").unwrap();
    let out = s.run(checked, &checker, extra);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty(), "{:?}", String::from_utf8_lossy(&out.stdout));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains(stderr_has), "{stderr}");
    assert!(!marker.exists(), "the checker ran");
    assert!(!s.record().exists());
}

#[test]
fn an_untracked_file_under_the_checked_path_is_refused() {
    let s = Scratch::new("def f(x):\n    return x\n");
    std::fs::create_dir_all(s.repo().join("app/sub")).unwrap();
    std::fs::write(s.repo().join("app/sub/new.py"), "x = 1\n").unwrap();
    assert_refused(&s, &s.repo().join("app"), &[], "app/sub/new.py");
}

#[test]
fn a_modified_tracked_file_is_refused() {
    let s = Scratch::new("def f(x):\n    return x\n");
    std::fs::write(s.repo().join("app/m.py"), "def f(x):\n    return x + 1\n").unwrap();
    assert_refused(&s, &s.repo().join("app"), &[], "app/m.py");
}

#[test]
fn a_path_outside_any_git_repository_is_refused() {
    let tmp = TempDir::new().unwrap();
    let s = Scratch::new("def f(x):\n    return x\n");
    let loose = tmp.path().join("loose");
    std::fs::create_dir_all(&loose).unwrap();
    std::fs::write(loose.join("m.py"), "def f(x):\n    return x\n").unwrap();
    assert_refused(&s, &loose, &[], "git work tree");
}

#[test]
fn an_overrides_file_outside_the_work_tree_is_refused() {
    let s = Scratch::new("def f(x):\n    return x\n");
    let overrides = s.tmp.path().join("o.toml");
    std::fs::write(&overrides, "").unwrap();
    assert_refused(&s, &s.repo().join("app"), &["--overrides", overrides.to_str().unwrap()], "outside the work tree");
}

#[test]
fn a_docstring_ensures_clause_adds_the_assumed_component() {
    let with = Scratch::new("def f(x):\n    \"\"\"ensures: precision(result) <= 4\"\"\"\n    return x\n");
    let out = with.run(&with.repo().join("app"), &with.checker(CLEAN_BODY), &[]);
    assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
    let rows = components(&with.read_record());
    assert_eq!(rows.len(), 8);
    assert_eq!(
        rows[7],
        (
            "Docstring requires: and ensures: contracts in the checked project, tagged ASSUMED and checked by no tool: 1".to_string(),
            with.head()
        )
    );

    let without = Scratch::new("def f(x):\n    return x\n");
    let out = without.run(&without.repo().join("app"), &without.checker(CLEAN_BODY), &[]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(components(&without.read_record()).len(), 7);
}

#[test]
fn a_run_without_the_option_creates_no_file() {
    let s = Scratch::new("def f(x):\n    return x\n");
    let cwd = s.tmp.path().join("cwd");
    std::fs::create_dir_all(&cwd).unwrap();
    let out = Command::new(cli())
        .current_dir(&cwd)
        .args(["contracts", "check"])
        .arg(s.repo().join("app"))
        .arg("--lean-checker")
        .arg(s.checker(CLEAN_BODY))
        .arg("--output-db")
        .arg(s.tmp.path().join("c.sqlite"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(out.stdout, CLEAN_JSON.as_bytes());
    assert_eq!(std::fs::read_dir(&cwd).unwrap().count(), 0);
    assert!(!s.record().exists());
    assert!(git(&s.repo(), &["status", "--porcelain"]).is_empty());
}

#[test]
fn a_record_that_cannot_be_written_exits_2_naming_the_path() {
    let s = Scratch::new("def f(x):\n    return x\n");
    let missing_dir = s.tmp.path().join("no-such-dir/record.json");
    let out = Command::new(cli())
        .args(["contracts", "check"])
        .arg(s.repo().join("app"))
        .arg("--lean-checker")
        .arg(s.checker(CLEAN_BODY))
        .arg("--output-db")
        .arg(s.tmp.path().join("c.sqlite"))
        .arg("--evidence-record")
        .arg(&missing_dir)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains(missing_dir.to_str().unwrap()));
}
