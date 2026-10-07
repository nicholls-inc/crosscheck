//! CLI behaviour around the checker process (round 3): parse errors, the
//! checker's output validation and exit codes, `--max-states` / `--max-paths`, and killing
//! the checker when the CLI is interrupted. A stand-in checker (a shell
//! script) replaces the Lean binary, so these tests need no Lean build.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use tempfile::TempDir;

fn cli() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_crosscheck-contracts"))
}

/// A stand-in checker: a `/bin/sh` script with `body`.
fn fake_checker(dir: &Path, body: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("checker.sh");
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn app(dir: &Path, source: &str) -> PathBuf {
    let app = dir.join("app");
    std::fs::create_dir_all(&app).unwrap();
    std::fs::write(app.join("m.py"), source).unwrap();
    app
}

fn run(app: &Path, checker: &Path, db: &Path, extra: &[&str]) -> Output {
    Command::new(cli())
        .args(["contracts", "check"])
        .arg(app)
        .arg("--lean-checker")
        .arg(checker)
        .arg("--output-db")
        .arg(db)
        .args(extra)
        .output()
        .expect("run crosscheck-contracts")
}

const VALID: &str = r#"{"summary": {"contracts_checked": 1, "edges_checked": 0, "paths_checked": 0}, "results": [], "exit_code": 1}"#;

#[test]
fn valid_checker_json_passes_through_byte_identical() {
    let tmp = TempDir::new().unwrap();
    let checker = fake_checker(tmp.path(), &format!("printf '%s' '{VALID}'\nexit 1"));
    let app = app(tmp.path(), "def f(x):\n    return x\n");
    let out = run(&app, &checker, &tmp.path().join("c.sqlite"), &[]);
    assert_eq!(out.stdout, VALID.as_bytes());
    assert_eq!(out.status.code(), Some(1));
    // Text format renders it.
    let out = run(&app, &checker, &tmp.path().join("c.sqlite"), &["--format", "text"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("RESULT: 0 errors, 0 warnings, 0 unverified. Exit code 1."));
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn invalid_checker_output_exits_2_in_both_formats() {
    let tmp = TempDir::new().unwrap();
    let app = app(tmp.path(), "def f(x):\n    return x\n");
    for body in [
        "echo 'INTERNAL PANIC: out of memory'\nexit 1",
        "exit 0",
        "printf '{\"summary\": '\nexit 0",
        "kill -9 $$",
    ] {
        let checker = fake_checker(tmp.path(), body);
        for format in ["json", "text"] {
            let out = run(&app, &checker, &tmp.path().join("c.sqlite"), &["--format", format]);
            assert_eq!(out.status.code(), Some(2), "{body} / {format}");
            assert!(out.stdout.is_empty(), "{body} / {format}: {:?}", out.stdout);
            assert!(
                String::from_utf8_lossy(&out.stderr).contains("did not produce a JSON result"),
                "{body} / {format}"
            );
        }
    }
}

#[test]
fn state_budgets_are_passed_after_the_database() {
    let tmp = TempDir::new().unwrap();
    let args_file = tmp.path().join("args.txt");
    let checker = fake_checker(
        tmp.path(),
        &format!("echo \"$@\" > '{}'\nprintf '%s' '{VALID}'\nexit 0", args_file.display()),
    );
    let app = app(tmp.path(), "def f(x):\n    return x\n");
    let db = tmp.path().join("c.sqlite");
    let out = run(&app, &checker, &db, &["--max-paths", "1234"]);
    assert_eq!(out.status.code(), Some(0));
    let args = std::fs::read_to_string(&args_file).unwrap();
    // `--max-paths` is an alias of `--max-states`.
    assert_eq!(args.trim(), format!("{} --max-states 1234", db.display()));
    run(&app, &checker, &db, &["--max-states", "7", "--max-states-per-edge", "3"]);
    let args = std::fs::read_to_string(&args_file).unwrap();
    assert_eq!(
        args.trim(),
        format!("{} --max-states 7 --max-states-per-edge 3", db.display())
    );
    // Without the flag, only the database.
    run(&app, &checker, &db, &[]);
    let args = std::fs::read_to_string(&args_file).unwrap();
    assert_eq!(args.trim(), db.display().to_string());
}

#[test]
fn parse_errors_fail_the_run_unless_allowed() {
    let tmp = TempDir::new().unwrap();
    let checker = fake_checker(tmp.path(), &format!("printf '%s' '{VALID}'\nexit 0"));
    let app = app(tmp.path(), "def ok(x):\n    return x\n");
    std::fs::write(app.join("broken.py"), "def f(:\n    pass\n").unwrap();
    let db = tmp.path().join("c.sqlite");
    let out = run(&app, &checker, &db, &[]);
    assert_eq!(out.status.code(), Some(2));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("broken.py: line 1"), "{err}");
    assert!(err.contains("--allow-parse-errors"), "{err}");
    assert!(out.stdout.is_empty());
    let out = run(&app, &checker, &db, &["--allow-parse-errors"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stderr).contains("skipped (syntax error) broken.py"));
    assert_eq!(out.stdout, VALID.as_bytes());
}

#[test]
fn deeply_nested_code_does_not_crash() {
    let tmp = TempDir::new().unwrap();
    let checker = fake_checker(tmp.path(), &format!("printf '%s' '{VALID}'\nexit 0"));
    let sum = vec!["x"; 20000].join(" + ");
    // (CPython itself rejects more than 200 nested parentheses.)
    let calls = format!("{}x{}", "g(".repeat(150), ")".repeat(150));
    let src = format!(
        "def g(value):\n    value = value.strip()\n    return value\ndef f(x):\n    return {sum}\ndef h(x):\n    return {calls}\n"
    );
    let app = app(tmp.path(), &src);
    let out = run(&app, &checker, &tmp.path().join("c.sqlite"), &[]);
    assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
}

/// An edge whose endpoint is not an extracted node is dropped with a warning
/// (not silently): one manual override names a function that does not exist.
#[test]
fn dropped_override_edge_is_reported() {
    let tmp = TempDir::new().unwrap();
    let checker = fake_checker(tmp.path(), &format!("printf '%s' '{VALID}'\nexit 0"));
    let app = app(tmp.path(), "def f(x):\n    return x\n\ndef g(y):\n    return f(y)\n");
    let overrides = tmp.path().join("overrides.toml");
    std::fs::write(
        &overrides,
        "[[edges]]\nsource = \"g\"\ntarget = \"f\"\nrelationship = \"calls\"\n\n\
         [[edges]]\nsource = \"g\"\ntarget = \"nowhere\"\nrelationship = \"calls\"\n",
    )
    .unwrap();
    let out = run(
        &app,
        &checker,
        &tmp.path().join("c.sqlite"),
        &["--overrides", overrides.to_str().unwrap()],
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("Warning: 1 manual override edge(s) dropped: an endpoint matches no extracted node"),
        "{err}"
    );
    assert!(!err.contains("discovered edge(s) dropped"), "{err}");
}

fn alive(pid: i32) -> bool {
    // Signal 0: existence check. A zombie still exists until reaped, so also
    // look at its state.
    let exists = unsafe { libc::kill(pid, 0) } == 0;
    let zombie = std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .map(|s| s.split_whitespace().nth(2) == Some("Z"))
        .unwrap_or(false);
    exists && !zombie
}

#[test]
fn interrupting_the_cli_kills_the_checker() {
    for signal in [libc::SIGTERM, libc::SIGINT] {
        let tmp = TempDir::new().unwrap();
        let pid_file = tmp.path().join("pid");
        // `exec sleep` so the checker's PID is the sleeping process.
        let checker = fake_checker(tmp.path(), &format!("echo $$ > '{}'\nexec sleep 60", pid_file.display()));
        let app = app(tmp.path(), "def f(x):\n    return x\n");
        let mut child = Command::new(cli())
            .args(["contracts", "check"])
            .arg(&app)
            .arg("--lean-checker")
            .arg(&checker)
            .arg("--output-db")
            .arg(tmp.path().join("c.sqlite"))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let start = Instant::now();
        let checker_pid = loop {
            if let Some(pid) = std::fs::read_to_string(&pid_file)
                .ok()
                .and_then(|s| s.trim().parse::<i32>().ok())
            {
                break pid;
            }
            assert!(start.elapsed() < Duration::from_secs(30), "checker never started");
            std::thread::sleep(Duration::from_millis(20));
        };
        assert!(alive(checker_pid));
        unsafe {
            libc::kill(child.id() as i32, signal);
        }
        let status = child.wait().unwrap();
        assert_eq!(status.code(), Some(128 + signal), "signal {signal}");
        let start = Instant::now();
        while alive(checker_pid) && start.elapsed() < Duration::from_secs(10) {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(!alive(checker_pid), "checker {checker_pid} survived signal {signal}");
    }
}
