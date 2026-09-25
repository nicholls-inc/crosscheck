use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crosscheck_contracts::{defaults, extractor, report};

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

        /// Text format only: omit WARNING blocks from the body (still counted in the RESULT line)
        #[arg(long)]
        no_warnings: bool,

        /// Path budget passed to the checker (`--max-paths N`); past it the
        /// checker reports an incomplete run (exit 2)
        #[arg(long, value_name = "N")]
        max_paths: Option<u64>,

        /// Skip files with syntax errors (with a warning) instead of failing the run
        #[arg(long)]
        allow_parse_errors: bool,
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
            max_paths,
            allow_parse_errors,
        } => {
            // Layer 1: extract contracts to SQLite (on a thread with a large stack)
            let options = extractor::ExtractOptions { allow_parse_errors };
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
            if let Some(n) = max_paths {
                command.arg("--max-paths").arg(n.to_string());
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
            match format {
                OutputFormat::Json => {
                    std::io::stdout().write_all(&output.stdout)?;
                    Ok(output.code.unwrap_or(2))
                }
                OutputFormat::Text => {
                    let stdout_str = String::from_utf8_lossy(&output.stdout);
                    match report::parse(&stdout_str) {
                        Ok(checker_output) => {
                            print!("{}", report::render_text(&checker_output, no_warnings));
                            Ok(checker_output.exit_code)
                        }
                        Err(e) => {
                            eprintln!("Error: unrecognised checker result ({e}):");
                            std::io::stderr().write_all(&output.stdout)?;
                            eprintln!();
                            Ok(2)
                        }
                    }
                }
            }
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
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    signals::set_child(child.id());
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
    use std::sync::atomic::{AtomicI32, Ordering};

    /// PID of the running checker, or 0.
    static CHILD: AtomicI32 = AtomicI32::new(0);

    extern "C" fn on_signal(sig: libc::c_int) {
        let pid = CHILD.load(Ordering::SeqCst);
        // Only async-signal-safe calls here: kill(2) and _exit(2).
        unsafe {
            if pid > 0 {
                libc::kill(pid, libc::SIGKILL);
            }
            libc::_exit(128 + sig);
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
}
