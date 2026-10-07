use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crosscheck_contracts::{baseline, defaults, evidence, extractor, report};

/// Stack size of the extraction thread: deeply nested Python (long operator
/// chains, nested calls) recurses in the parser and the analyses.
const EXTRACTION_STACK: usize = 256 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    Json,
    Text,
}

#[derive(Parser)]
#[command(name = "crosscheck")]
#[command(about = "Contract graph verifier for Python applications (Django models, dataclasses, attrs, pydantic)")]
struct Cli {
    #[command(subcommand)]
    command: TopCommands,
}

#[derive(Subcommand)]
enum TopCommands {
    /// Contract verification commands
    Contracts {
        #[command(subcommand)]
        command: Commands,
    },
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant, reason = "parsed once per process")]
enum Commands {
    /// Check contracts in a Python application
    Check {
        /// Path to the application directory (or a single .py file)
        app_path: PathBuf,

        /// Path to manual edge overrides file
        #[arg(long)]
        overrides: Option<PathBuf>,

        /// Django version for defaults (e.g., "4.2")
        #[arg(long, default_value = "4.2")]
        django_version: String,

        /// Path to the Lean checker binary
        #[arg(long)]
        lean_checker: Option<PathBuf>,

        /// Where to write the SQLite contract database (default: <tmp>/crosscheck/contracts.sqlite)
        #[arg(long)]
        output_db: Option<PathBuf>,

        /// Output format: JSON (machine-readable, default) or text (human-readable)
        #[arg(long, value_enum, default_value = "json")]
        format: OutputFormat,

        /// Text format only: omit WARNING and UNVERIFIED blocks from the body (still counted in the RESULT line)
        #[arg(long, conflicts_with = "warnings")]
        no_warnings: bool,

        /// Text format only: also list each UNVERIFIED requirement (a hop whose
        /// source has no guarantee of the kind its target requires). By default
        /// they are only counted, in the coverage section and the RESULT line
        #[arg(long)]
        warnings: bool,

        /// Search budget passed to the checker (`--max-states N`, total hop
        /// states); past it the checker reports an incomplete run (exit 2).
        /// `--max-paths` is an alias.
        #[arg(long, value_name = "N", alias = "max-paths")]
        max_states: Option<u64>,

        /// Per-edge state budget passed to the checker (`--max-states-per-edge N`)
        #[arg(long, value_name = "N")]
        max_states_per_edge: Option<u64>,

        /// Skip files whose path relative to the application root (or one of
        /// its directories) matches GLOB (`*`, `?`, `**`); repeatable
        #[arg(long, value_name = "GLOB")]
        exclude: Vec<String>,

        /// Skip files with syntax errors (with a warning) instead of failing the run
        #[arg(long)]
        allow_parse_errors: bool,

        /// After a run that exits 0, write an evidence record (JSON) to PATH.
        /// Any other outcome removes the file. Needs a clean git checkout of
        /// the checked path and the overrides file.
        #[arg(long, value_name = "PATH")]
        evidence_record: Option<PathBuf>,

        /// Write the findings of this run to a baseline file at PATH. A run
        /// that is incomplete or fails writes no file. Output and exit code
        /// are unchanged
        #[arg(long, value_name = "PATH")]
        write_baseline: Option<PathBuf>,

        /// Compare the findings with the baseline file at PATH: show only the
        /// new ones, count the others, and exit 1 when an error is new. It
        /// cannot name the same file as --write-baseline
        #[arg(long, value_name = "PATH", conflicts_with = "evidence_record")]
        baseline: Option<PathBuf>,
    },
    /// Generate defaults table from Django source
    GenerateDefaults {
        /// Path to Django source tree
        #[arg(long)]
        django_source: PathBuf,

        /// Django version identifier
        #[arg(long)]
        version: String,
    },
}

fn main() {
    signals::install();
    let cli = Cli::parse();
    let code = match run(cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {e:#}");
            2
        }
    };
    std::process::exit(code);
}

fn run(cli: Cli) -> Result<i32> {
    let TopCommands::Contracts { command } = cli.command;
    match command {
        Commands::Check {
            app_path,
            overrides,
            django_version,
            lean_checker,
            output_db,
            format,
            no_warnings,
            warnings,
            max_states,
            max_states_per_edge,
            exclude,
            allow_parse_errors,
            evidence_record,
            write_baseline,
            baseline: baseline_path,
        } => {
            if let (Some(read), Some(write)) = (&baseline_path, &write_baseline) {
                if baseline::same_file(read, write) {
                    eprintln!(
                        "error: --baseline and --write-baseline name the same file {}; overwriting the baseline with this run's findings would let the next run match its new errors",
                        read.display()
                    );
                    return Ok(2);
                }
            }
            let baseline_findings = baseline_path.as_deref().map(baseline::read);
            if let Some(path) = &write_baseline {
                baseline::remove(path)?;
            }
            let baseline_findings = baseline_findings.transpose()?;
            let root = baseline::root(&app_path);
            let checkout = evidence_record
                .as_deref()
                .map(|record_path| evidence::begin(record_path, &app_path, overrides.as_deref(), &exclude))
                .transpose()?;
            let rerun_options = (exclude.clone(), django_version.clone());

            // Layer 1: extract contracts to SQLite (on a thread with a large stack)
            let options = extractor::ExtractOptions {
                allow_parse_errors,
                exclude,
            };
            let db_path = std::thread::Builder::new()
                .name("extract".into())
                .stack_size(EXTRACTION_STACK)
                .spawn(move || {
                    extractor::extract_with(
                        &app_path,
                        overrides.as_deref(),
                        &django_version,
                        output_db.as_deref(),
                        &options,
                    )
                })?
                .join()
                .map_err(|_| anyhow::anyhow!("extraction panicked"))??;

            // Layer 2+3: invoke Lean checker
            let lean_binary = lean_checker.or_else(find_lean_binary).ok_or_else(|| {
                anyhow::anyhow!("Lean checker binary not found. Use --lean-checker to specify path.")
            })?;
            let mut command = Command::new(&lean_binary);
            command.arg(&db_path);
            if let Some(n) = max_states {
                command.arg("--max-states").arg(n.to_string());
            }
            if let Some(n) = max_states_per_edge {
                command.arg("--max-states-per-edge").arg(n.to_string());
            }
            let output = run_checker(command).map_err(|e| {
                anyhow::anyhow!("Failed to run Lean checker at {}: {e}", lean_binary.display())
            })?;

            // Report results
            if !output.stderr.is_empty() {
                std::io::stderr().write_all(&output.stderr)?;
            }
            let is_json = serde_json::from_slice::<serde_json::Value>(&output.stdout).is_ok();
            if !is_json {
                // Not a checker result (crash, out of memory, killed): the run is incomplete.
                eprintln!(
                    "Error: the checker did not produce a JSON result (exit status {}); the run is incomplete.",
                    output
                        .code
                        .map_or("killed by a signal".to_string(), |c| c.to_string())
                );
                if !output.stdout.is_empty() {
                    eprintln!("Checker output:");
                    std::io::stderr().write_all(&output.stdout)?;
                    eprintln!();
                }
                return Ok(2);
            }
            let parsed = (baseline_findings.is_some() || write_baseline.is_some())
                .then(|| report::parse(&String::from_utf8_lossy(&output.stdout)));
            if let (Some(Err(e)), Some(_)) = (&parsed, &baseline_findings) {
                eprintln!("Error: unrecognised checker result ({e}):");
                std::io::stderr().write_all(&output.stdout)?;
                eprintln!();
                return Ok(2);
            }
            if let Some(Ok(checker_output)) = parsed {
                let keys = baseline::keys(&checker_output, &root)?;
                let incomplete = keys.iter().any(Option::is_none);
                // A JSON exit code the process exit status contradicts is not a completed run.
                let checker_code = if output.code == Some(checker_output.exit_code) {
                    checker_output.exit_code
                } else {
                    2
                };
                if let Some(path) = &write_baseline {
                    if !incomplete && (checker_code == 0 || checker_code == 1) {
                        std::fs::write(path, baseline::render(&keys))
                            .map_err(|e| anyhow::anyhow!("cannot write the baseline {}: {e}", path.display()))?;
                    }
                }
                if let Some(findings) = &baseline_findings {
                    let diff = baseline::diff(&keys, findings);
                    let errors = diff.new.errors + diff.existing.errors;
                    let code = baseline::exit_code(checker_code, incomplete, diff.new.errors, errors);
                    match format {
                        OutputFormat::Json => {
                            let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
                            println!("{}", baseline::render_json(&json, &diff, code));
                        }
                        OutputFormat::Text => {
                            let edges = report::edges_by_module(&db_path).map_err(|e| {
                                anyhow::anyhow!("cannot count the edges of {}: {e}", db_path.display())
                            })?;
                            let display = warning_display(no_warnings, warnings);
                            print!(
                                "{}",
                                report::render_text_with(&checker_output, display, &edges, Some((&diff, code)))
                            );
                        }
                    }
                    return Ok(code);
                }
            }
            let code = match format {
                OutputFormat::Json => {
                    std::io::stdout().write_all(&output.stdout)?;
                    output.code.unwrap_or(2)
                }
                OutputFormat::Text => {
                    let stdout_str = String::from_utf8_lossy(&output.stdout);
                    match report::parse(&stdout_str) {
                        Ok(checker_output) => {
                            let display = warning_display(no_warnings, warnings);
                            let edges = report::edges_by_module(&db_path).map_err(|e| {
                                anyhow::anyhow!("cannot count the edges of {}: {e}", db_path.display())
                            })?;
                            print!("{}", report::render_text(&checker_output, display, &edges));
                            checker_output.exit_code
                        }
                        Err(e) => {
                            eprintln!("Error: unrecognised checker result ({e}):");
                            std::io::stderr().write_all(&output.stdout)?;
                            eprintln!();
                            2
                        }
                    }
                }
            };
            if let (0, Some(checkout), Some(record_path)) = (code, checkout, evidence_record) {
                let checker_output = report::parse(&String::from_utf8_lossy(&output.stdout))
                    .map_err(|e| anyhow::anyhow!("cannot count warnings in the checker result: {e}"))?;
                let checker = lean_binary.canonicalize()?;
                let checker_path = checker.to_string_lossy();
                let (exclude, django_version) = rerun_options;
                let record = evidence::build_record(&evidence::RecordInputs {
                    commit: &checkout.commit,
                    warnings: checker_output.warning_count(),
                    assumed: evidence::count_assumed(&db_path)?,
                    checker_sha256: &evidence::sha256_hex(&checker)?,
                    rerun_command: evidence::rerun_command(&evidence::RerunOptions {
                        path: &checkout.path,
                        django_version: &django_version,
                        overrides: checkout.overrides.as_deref(),
                        exclude: &exclude,
                        allow_parse_errors,
                        max_states,
                        max_states_per_edge,
                        checker: &checker_path,
                    }),
                });
                evidence::recheck(&checkout)?;
                evidence::write(&record_path, &record)?;
            }
            Ok(code)
        }
        Commands::GenerateDefaults {
            django_source,
            version,
        } => {
            defaults::generate_defaults(&django_source, &version)?;
            Ok(0)
        }
    }
}

fn warning_display(no_warnings: bool, warnings: bool) -> report::WarningDisplay {
    match (no_warnings, warnings) {
        (true, _) => report::WarningDisplay::Hide,
        (_, true) => report::WarningDisplay::All,
        _ => report::WarningDisplay::Default,
    }
}

/// What the checker printed and how it exited (`code` is `None` when it was
/// killed by a signal).
struct CheckerRun {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
}

/// Run the checker, collecting its output. While it runs, an interrupt
/// (SIGINT / SIGTERM / SIGHUP) to this process kills it too.
fn run_checker(mut command: Command) -> std::io::Result<CheckerRun> {
    signals::begin_spawn();
    let spawned = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match spawned {
        Ok(child) => child,
        Err(e) => {
            signals::end_spawn(None);
            return Err(e);
        }
    };
    signals::end_spawn(Some(child.id()));
    let mut out = child.stdout.take().expect("piped stdout");
    let mut err = child.stderr.take().expect("piped stderr");
    let err_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = err.read_to_end(&mut buf);
        buf
    });
    let mut stdout = Vec::new();
    let read = out.read_to_end(&mut stdout);
    let status = child.wait();
    signals::set_child(0);
    let stderr = err_reader.join().unwrap_or_default();
    read?;
    Ok(CheckerRun {
        stdout,
        stderr,
        code: status?.code(),
    })
}

/// Try to find the Lean checker binary relative to the current executable.
fn find_lean_binary() -> Option<PathBuf> {
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    let candidate = exe_dir.join("contract-graph-checker");
    if candidate.exists() {
        Some(candidate)
    } else {
        None
    }
}

/// Interrupt handling: a signal that terminates the CLI also kills the
/// running checker, so no orphan keeps computing (the checker can run for
/// minutes and use gigabytes).
#[cfg(unix)]
mod signals {
    use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

    /// PID of the running checker, or 0.
    static CHILD: AtomicI32 = AtomicI32::new(0);
    /// Between `begin_spawn` and `end_spawn`: a child may exist whose PID is
    /// not yet in `CHILD`.
    static SPAWNING: AtomicBool = AtomicBool::new(false);
    /// A signal that arrived while `SPAWNING` (0: none).
    static PENDING: AtomicI32 = AtomicI32::new(0);

    extern "C" fn on_signal(sig: libc::c_int) {
        let pid = CHILD.load(Ordering::SeqCst);
        // Only async-signal-safe calls here: atomics, kill(2) and _exit(2).
        unsafe {
            if pid > 0 {
                libc::kill(pid, libc::SIGKILL);
            } else if SPAWNING.load(Ordering::SeqCst) {
                // The child may already exist with its PID unrecorded:
                // `end_spawn` kills it and exits.
                PENDING.store(sig, Ordering::SeqCst);
                return;
            }
            libc::_exit(128 + sig);
        }
    }

    pub fn begin_spawn() {
        SPAWNING.store(true, Ordering::SeqCst);
    }

    /// Record the spawned child (if any), then act on a signal that arrived
    /// during the spawn. `CHILD` is set before `SPAWNING` is cleared, so a
    /// signal at any point either sees the PID or is left in `PENDING`.
    pub fn end_spawn(pid: Option<u32>) {
        if let Some(pid) = pid {
            set_child(pid);
        }
        SPAWNING.store(false, Ordering::SeqCst);
        let sig = PENDING.swap(0, Ordering::SeqCst);
        if sig != 0 {
            unsafe {
                if let Some(pid) = pid {
                    libc::kill(pid as i32, libc::SIGKILL);
                }
                libc::_exit(128 + sig);
            }
        }
    }

    pub fn install() {
        let handler = on_signal as extern "C" fn(libc::c_int) as libc::sighandler_t;
        for sig in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP] {
            unsafe {
                libc::signal(sig, handler);
            }
        }
    }

    pub fn set_child(pid: u32) {
        CHILD.store(pid as i32, Ordering::SeqCst);
    }
}

#[cfg(not(unix))]
mod signals {
    pub fn install() {}
    pub fn set_child(_pid: u32) {}
    pub fn begin_spawn() {}
    pub fn end_spawn(_pid: Option<u32>) {}
}
