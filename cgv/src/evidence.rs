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
                "ASSUMED contracts in the checked project: {} docstring ensures: clauses that no tool checked",
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
pub fn begin(record_path: &Path, app_path: &Path, overrides: Option<&Path>) -> Result<Checkout> {
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
    let mut status = vec!["status", "--porcelain", "--untracked-files=all", "--", path.as_str()];
    status.extend(overrides_rel.as_deref());
    let changes = git(&top, &status)?;
    if !changes.is_empty() {
        bail!(
            "--evidence-record needs a clean checkout of the checked path and overrides file, but git reports changes:\n{changes}"
        );
    }
    let commit = git(&top, &["rev-parse", "HEAD"])?;
    Ok(Checkout { commit, path, overrides: overrides_rel })
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
    std::fs::write(record_path, json)
        .with_context(|| format!("cannot write the evidence record to {}", record_path.display()))
}
