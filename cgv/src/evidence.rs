//! The evidence record (`intent/2026-10-06-evidence-record-spec.md`) that
//! `contracts check --evidence-record PATH` writes after a run that exits 0.
//! The pure builders come first. Git and file access sit under "Boundary".

use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;

const FORMAT: &str = "evidence-record/1";
const CLAIM_ID: &str = "cgv-data-paths";
const THEOREM: &str = "ContractGraph.runChecker_sound_all";
const TOOLCHAIN: &str = include_str!("../prover/lean-toolchain");
const BUILD_COMMIT: &str = env!("CGV_BUILD_COMMIT");

#[derive(Debug, Serialize)]
pub struct Record {
    pub format: String,
    pub commit: String,
    pub claims: Vec<Claim>,
}

#[derive(Debug, Serialize)]
pub struct Claim {
    pub id: String,
    pub statement: String,
    pub requirement: Option<String>,
    pub strength: String,
    pub basis: Basis,
    pub trusted_base: Vec<Component>,
    pub rerun: Rerun,
}

#[derive(Debug, Serialize)]
pub struct Basis {
    pub theorems: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Component {
    pub component: String,
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct Rerun {
    pub command: String,
    pub exit_code: i32,
}

pub struct RecordInputs<'a> {
    pub commit: &'a str,
    pub warnings: usize,
    pub assumed: u64,
    pub checker_sha256: &'a str,
    pub rerun_command: String,
}

/// The options of a run that its rerun command repeats.
pub struct RerunOptions<'a> {
    pub path: &'a str,
    pub django_version: &'a str,
    pub overrides: Option<&'a str>,
    pub exclude: &'a [String],
    pub allow_parse_errors: bool,
    pub max_states: Option<u64>,
    pub max_states_per_edge: Option<u64>,
    pub checker: &'a str,
}

pub fn build_record(inputs: &RecordInputs) -> Record {
    Record {
        format: FORMAT.to_string(),
        commit: inputs.commit.to_string(),
        claims: vec![Claim {
            id: CLAIM_ID.to_string(),
            statement: statement(inputs.warnings),
            requirement: None,
            strength: "proved".to_string(),
            basis: Basis {
                theorems: vec![THEOREM.to_string()],
            },
            trusted_base: trusted_base(inputs),
            rerun: Rerun {
                command: inputs.rerun_command.clone(),
                exit_code: 0,
            },
        }],
    }
}

fn statement(warnings: usize) -> String {
    format!(
        "At every hop of every checked data path from a function to a model field, the source's guarantees imply the target's requirements, for the contracts the CGV extractor derived from the checked path at that commit. The checker reported {warnings} warning{}. A warning marks a requirement CGV could not show, which the theorem does not cover.",
        if warnings == 1 { "" } else { "s" }
    )
}

fn trusted_base(inputs: &RecordInputs) -> Vec<Component> {
    let toolchain = TOOLCHAIN.trim();
    let checker_hash = format!("sha256:{}", inputs.checker_sha256);
    let mut rows = vec![
        ("Lean 4 kernel", toolchain),
        (
            "Lean axioms propext, Classical.choice and Quot.sound",
            toolchain,
        ),
        (
            "Lean compiler and runtime that built contract-graph-checker",
            toolchain,
        ),
        ("contract-graph-checker binary", checker_hash.as_str()),
        ("CGV Rust extractor and command line", BUILD_COMMIT),
        ("CGV Translation.lean", BUILD_COMMIT),
        ("CGV BehaviorModel.lean", BUILD_COMMIT),
    ]
    .into_iter()
    .map(|(component, version)| Component {
        component: component.to_string(),
        version: version.to_string(),
    })
    .collect::<Vec<_>>();
    if inputs.assumed >= 1 {
        rows.push(Component {
            component: format!(
                "Docstring requires: and ensures: contracts in the checked project, tagged ASSUMED and checked by no tool: {}",
                inputs.assumed
            ),
            version: inputs.commit.to_string(),
        });
    }
    rows
}

/// One word of POSIX shell: single-quoted unless every character is safe.
pub fn shell_quote(word: &str) -> String {
    let safe = |c: char| c.is_ascii_alphanumeric() || "_./=:@%+-".contains(c);
    if !word.is_empty() && word.chars().all(safe) {
        word.to_string()
    } else {
        format!("'{}'", word.replace('\'', "'\\''"))
    }
}

pub fn rerun_command(o: &RerunOptions) -> String {
    let mut words = vec![
        "crosscheck-contracts".to_string(),
        "contracts".to_string(),
        "check".to_string(),
        o.path.to_string(),
        "--django-version".to_string(),
        o.django_version.to_string(),
    ];
    if let Some(p) = o.overrides {
        words.extend(["--overrides".to_string(), p.to_string()]);
    }
    for glob in o.exclude {
        words.extend(["--exclude".to_string(), glob.clone()]);
    }
    if o.allow_parse_errors {
        words.push("--allow-parse-errors".to_string());
    }
    if let Some(n) = o.max_states {
        words.extend(["--max-states".to_string(), n.to_string()]);
    }
    if let Some(n) = o.max_states_per_edge {
        words.extend(["--max-states-per-edge".to_string(), n.to_string()]);
    }
    words.extend(["--lean-checker".to_string(), o.checker.to_string()]);
    words
        .iter()
        .map(|w| shell_quote(w))
        .collect::<Vec<_>>()
        .join(" ")
}

// Boundary: git and the file system.

/// The work tree that holds the checked path, as `begin` found it.
pub struct Checkout {
    pub commit: String,
    /// The checked path relative to the work-tree root (`.` for the root).
    pub path: String,
    pub overrides: Option<String>,
    top: PathBuf,
    app: PathBuf,
    exclude: Vec<String>,
}

fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("--literal-pathspecs")
        .args(args)
        .output()
        .context("failed to run git")?;
    if !out.status.success() {
        bail!("{}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
}

fn relative(top: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(top).ok()?;
    Some(if rel.as_os_str().is_empty() {
        ".".to_string()
    } else {
        rel.to_string_lossy().into_owned()
    })
}

/// Remove any file at `record_path` so that no failing run leaves one, then
/// refuse a checked path that git cannot pin to a commit (CR-3).
pub fn begin(
    record_path: &Path,
    app_path: &Path,
    overrides: Option<&Path>,
    exclude: &[String],
) -> Result<Checkout> {
    match std::fs::remove_file(record_path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            return Err(e).with_context(|| format!("cannot remove {}", record_path.display()))
        }
        _ => {}
    }
    let app = app_path
        .canonicalize()
        .with_context(|| format!("cannot read {}", app_path.display()))?;
    let dir = if app.is_dir() { app.clone() } else { app.parent().unwrap_or(&app).to_path_buf() };
    let top = PathBuf::from(
        git(&dir, &["rev-parse", "--show-toplevel"])
            .map_err(|e| anyhow!("--evidence-record needs the checked path to be in a git work tree ({e})"))?,
    )
    .canonicalize()?;
    let overrides_abs = overrides
        .map(|p| p.canonicalize().with_context(|| format!("cannot read {}", p.display())))
        .transpose()?;
    let path = relative(&top, &app).ok_or_else(|| anyhow!("{} is outside its own work tree", app.display()))?;
    let overrides_rel = match &overrides_abs {
        Some(p) => Some(relative(&top, p).ok_or_else(|| {
            anyhow!(
                "--overrides file {} is outside the work tree {}; --evidence-record needs it in the same work tree",
                p.display(),
                top.display()
            )
        })?),
        None => None,
    };
    let mut checkout = Checkout {
        commit: String::new(),
        path,
        overrides: overrides_rel,
        top,
        app,
        exclude: exclude.to_vec(),
    };
    checkout.commit = clean_commit(&checkout)?;
    Ok(checkout)
}

/// Repeat the checks of `begin` just before the record is written, and refuse
/// if the checkout changed or HEAD moved while the run was going (CR-3).
pub fn recheck(checkout: &Checkout) -> Result<()> {
    let commit = clean_commit(checkout).context("the checkout changed during the run")?;
    if commit != checkout.commit {
        bail!(
            "--evidence-record refuses: HEAD moved from {} to {commit} during the run",
            checkout.commit
        );
    }
    Ok(())
}

/// HEAD, once git reports no change under the checked path or the overrides
/// file and no ignored .py file the run would analyse.
fn clean_commit(c: &Checkout) -> Result<String> {
    let mut status = vec!["status", "--porcelain", "--untracked-files=all", "--", c.path.as_str()];
    status.extend(c.overrides.as_deref());
    let changes = git(&c.top, &status)?;
    if !changes.is_empty() {
        bail!(
            "--evidence-record needs a clean checkout of the checked path and overrides file, but git reports changes:\n{changes}"
        );
    }
    let ignored = ignored_python_files(c)?;
    if !ignored.is_empty() {
        bail!(
            "--evidence-record refuses: git ignores these .py files, which the run would analyse, so the commit does not pin them (remove them or pass --exclude):\n{}",
            ignored.join("\n")
        );
    }
    git(&c.top, &["rev-parse", "HEAD"])
}

/// The files the run analyses that git ignores, relative to the work-tree root.
fn ignored_python_files(c: &Checkout) -> Result<Vec<String>> {
    let listed = git(
        &c.top,
        &["ls-files", "-z", "--others", "--ignored", "--exclude-standard", "--", c.path.as_str()],
    )?;
    let ignored: std::collections::HashSet<&str> = listed.split('\0').filter(|f| f.ends_with(".py")).collect();
    if ignored.is_empty() {
        return Ok(Vec::new());
    }
    Ok(crate::extractor::python_files(&c.app, &c.exclude)?
        .iter()
        .filter_map(|f| relative(&c.top, f))
        .filter(|f| ignored.contains(f.as_str()))
        .collect())
}

pub fn sha256_hex(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path).with_context(|| format!("cannot read {}", path.display()))?;
    Ok(Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect())
}

pub fn count_assumed(db: &Path) -> Result<u64> {
    let conn = rusqlite::Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM contracts WHERE verification_level = 'ASSUMED'",
        [],
        |row| row.get(0),
    )?)
}

pub fn write(record_path: &Path, record: &Record) -> Result<()> {
    let mut json = serde_json::to_string_pretty(record)?;
    json.push('\n');
    write_through_temp(record_path, |file| std::io::Write::write_all(file, json.as_bytes()))
        .with_context(|| format!("cannot write the evidence record to {}", record_path.display()))
}

/// Fill a temporary file beside `record_path`, then rename it into place, so a
/// failed write never leaves a partial record at `record_path` (CR-2).
fn write_through_temp(
    record_path: &Path,
    fill: impl FnOnce(&mut std::fs::File) -> std::io::Result<()>,
) -> std::io::Result<()> {
    let name = record_path.file_name().ok_or_else(|| std::io::Error::other("no file name"))?;
    let temp = record_path.with_file_name(format!(".{}.tmp-{}", name.to_string_lossy(), std::process::id()));
    let result = std::fs::File::create(&temp)
        .and_then(|mut file| fill(&mut file).and_then(|()| file.sync_all()))
        .and_then(|()| std::fs::rename(&temp, record_path));
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

#[cfg(test)]
mod write_tests {
    use super::write_through_temp;
    use std::io::Write;

    #[test]
    fn a_failed_fill_leaves_neither_a_record_nor_a_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let record = dir.path().join("record.json");
        let result = write_through_temp(&record, |file| {
            file.write_all(b"{\"partial\":")?;
            Err(std::io::Error::other("disk full"))
        });
        assert!(result.is_err());
        let left: Vec<_> = std::fs::read_dir(dir.path()).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert!(left.is_empty(), "left behind: {left:?}");
    }

    #[test]
    fn a_failed_fill_keeps_nothing_of_an_older_record_the_caller_removed_and_a_good_fill_replaces_it() {
        let dir = tempfile::tempdir().unwrap();
        let record = dir.path().join("record.json");
        write_through_temp(&record, |f| f.write_all(b"one")).unwrap();
        write_through_temp(&record, |f| f.write_all(b"two")).unwrap();
        assert_eq!(std::fs::read(&record).unwrap(), b"two");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
