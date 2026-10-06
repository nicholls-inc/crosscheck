use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn main() {
    let commit = match git(&["rev-parse", "HEAD"]) {
        Some(sha) => match git(&["status", "--porcelain", "--", "."]) {
            Some(changes) if changes.is_empty() => sha,
            Some(_) => format!("{sha}-dirty"),
            None => "unknown".to_string(),
        },
        None => "unknown".to_string(),
    };
    println!("cargo:rustc-env=CGV_BUILD_COMMIT={commit}");

    for path in ["src", "prover/ContractGraph", "Cargo.toml", "build.rs"] {
        println!("cargo:rerun-if-changed={path}");
    }
    if let Some(head) = git(&["rev-parse", "--git-path", "HEAD"]) {
        println!("cargo:rerun-if-changed={head}");
    }
    if let Some(reference) = git(&["rev-parse", "--symbolic-full-name", "HEAD"]) {
        if let Some(file) = git(&["rev-parse", "--git-path", &reference]) {
            println!("cargo:rerun-if-changed={file}");
        }
    }
}
