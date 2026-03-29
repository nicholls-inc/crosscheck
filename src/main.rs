use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crosscheck_contracts::{defaults, extractor};

#[derive(Parser)]
#[command(name = "crosscheck")]
#[command(about = "Contract graph verifier for Django applications")]
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
    /// Check contracts in a Django application
    Check {
        /// Path to the Django application directory
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

fn main() -> Result<()> {
    let cli = Cli::parse();

    let TopCommands::Contracts { command } = cli.command;
    match command {
        Commands::Check {
            app_path,
            overrides,
            django_version,
            lean_checker,
        } => {
            // Layer 1: extract contracts to SQLite
            let db_path = extractor::extract(&app_path, overrides.as_deref(), &django_version, None)?;

            // Layer 2+3: invoke Lean checker
            let lean_binary = lean_checker
                .or_else(find_lean_binary)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Lean checker binary not found. Use --lean-checker to specify path."
                    )
                })?;

            let output = std::process::Command::new(&lean_binary)
                .arg(&db_path)
                .output()
                .map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to run Lean checker at {}: {}",
                        lean_binary.display(),
                        e
                    )
                })?;

            // Report results
            std::io::Write::write_all(&mut std::io::stdout(), &output.stdout)?;
            if !output.stderr.is_empty() {
                std::io::Write::write_all(&mut std::io::stderr(), &output.stderr)?;
            }

            let exit_code = output.status.code().unwrap_or(2);
            std::process::exit(exit_code);
        }
        Commands::GenerateDefaults {
            django_source,
            version,
        } => {
            defaults::generate_defaults(&django_source, &version)?;
        }
    }

    Ok(())
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
