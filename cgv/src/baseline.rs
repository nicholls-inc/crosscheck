//! Baseline mode of `contracts check` (spec:
//! `intent/2026-10-07-cgv-baseline-mode-spec.md`, BL-1 to BL-11): match the
//! results of a run with the findings of a baseline file, so a report shows
//! only the findings a change introduced.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::report::CheckerOutput;

pub const FORMAT: &str = "cgv-baseline/1";

/// The key of a result (BL-3). It holds no line number, so an edit above a
/// finding leaves its key unchanged.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Key {
    pub class: String,
    pub source: String,
    pub target: String,
    pub hop: Vec<String>,
    pub source_guarantee: String,
    pub target_requirement: String,
    pub site_file: String,
    /// Required in a baseline file even when null: serde treats a missing
    /// `Option` field as `None` unless it is deserialized explicitly.
    #[serde(deserialize_with = "Option::deserialize")]
    pub site_text: Option<String>,
}

/// A baseline file (BL-4).
#[derive(Debug, Serialize, Deserialize)]
pub struct BaselineFile {
    pub format: String,
    pub findings: Vec<Key>,
}

/// How a result of the run compares with the baseline (BL-7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Incomplete,
    New,
    Existing,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Counts {
    pub errors: usize,
    pub warnings: usize,
    pub unverified: usize,
}

impl Counts {
    fn add(&mut self, class: &str) {
        match class {
            "error" => self.errors += 1,
            "warning" => self.warnings += 1,
            _ => self.unverified += 1,
        }
    }
}

/// The comparison of a run with a baseline. `status` is parallel to the
/// run's `results`.
#[derive(Debug, PartialEq, Eq)]
pub struct Diff {
    pub status: Vec<Status>,
    pub fixed: Vec<Key>,
    pub new: Counts,
    pub existing: Counts,
}

/// The root of a run (BL-2).
pub fn root(app_path: &Path) -> PathBuf {
    if app_path.is_dir() {
        app_path.to_path_buf()
    } else {
        app_path.parent().unwrap_or(Path::new("")).to_path_buf()
    }
}

fn normalise(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The key of every result, `None` for an incomplete one (BL-3). Fails,
/// naming the file, when a site line cannot be read.
pub fn keys(output: &CheckerOutput, root: &Path) -> Result<Vec<Option<Key>>> {
    let mut files: BTreeMap<String, Vec<String>> = BTreeMap::new();
    output
        .results
        .iter()
        .map(|result| {
            let Some(class) = result.class_name() else { return Ok(None) };
            let (site_file, line) = match &result.site {
                Some(site) => (site.file.clone(), site.line),
                None => (String::new(), 0),
            };
            let site_text = if site_file.is_empty() {
                None
            } else {
                if !files.contains_key(&site_file) {
                    let path = root.join(&site_file);
                    let bytes = std::fs::read(&path)
                        .with_context(|| format!("cannot read the site file {}", path.display()))?;
                    let lines = String::from_utf8_lossy(&bytes).lines().map(normalise).collect();
                    files.insert(site_file.clone(), lines);
                }
                let lines = &files[&site_file];
                let text = usize::try_from(line)
                    .ok()
                    .and_then(|n| n.checked_sub(1))
                    .and_then(|i| lines.get(i));
                match text {
                    Some(text) => Some(text.clone()),
                    None => bail!(
                        "the site file {} has no line {line} ({} lines)",
                        root.join(&site_file).display(),
                        lines.len()
                    ),
                }
            };
            Ok(Some(Key {
                class: class.to_string(),
                source: result.source.name.clone(),
                target: result.target.name.clone(),
                hop: result.hop.clone(),
                source_guarantee: result.source_guarantee.clone(),
                target_requirement: result.target_requirement.clone(),
                site_file,
                site_text,
            }))
        })
        .collect()
}

/// Parse a baseline file into the number of findings with each key (BL-6).
pub fn parse(json: &str) -> Result<BTreeMap<Key, usize>> {
    let file: BaselineFile = serde_json::from_str(json)?;
    if file.format != FORMAT {
        bail!("format is {:?}, expected {FORMAT:?}", file.format);
    }
    let mut counts = BTreeMap::new();
    for key in file.findings {
        *counts.entry(key).or_insert(0) += 1;
    }
    Ok(counts)
}

pub fn read(path: &Path) -> Result<BTreeMap<Key, usize>> {
    let json = std::fs::read_to_string(path).with_context(|| format!("cannot read the baseline {}", path.display()))?;
    parse(&json).with_context(|| format!("invalid baseline {}", path.display()))
}

/// The baseline file of `keys`, findings sorted (BL-4).
pub fn render(keys: &[Option<Key>]) -> String {
    let mut findings: Vec<Key> = keys.iter().flatten().cloned().collect();
    findings.sort();
    let file = BaselineFile { format: FORMAT.to_string(), findings };
    serde_json::to_string_pretty(&file).expect("a baseline serializes") + "\n"
}

/// Remove any file at `path` (BL-5).
pub fn remove(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            Err(e).with_context(|| format!("cannot remove {}", path.display()))
        }
        _ => Ok(()),
    }
}

/// Whether `a` and `b` name the same file (BL-1). A path that does not exist
/// yet is compared by its canonical parent directory and its file name.
pub fn same_file(a: &Path, b: &Path) -> bool {
    fn canonical(p: &Path) -> Option<PathBuf> {
        if let Ok(c) = p.canonicalize() {
            return Some(c);
        }
        let parent = match p.parent() {
            Some(d) if !d.as_os_str().is_empty() => d,
            _ => Path::new("."),
        };
        Some(parent.canonicalize().ok()?.join(p.file_name()?))
    }
    match (canonical(a), canonical(b)) {
        (Some(x), Some(y)) => x == y,
        _ => a == b,
    }
}

/// Match the keys of a run with a baseline (BL-7).
pub fn diff(keys: &[Option<Key>], baseline: &BTreeMap<Key, usize>) -> Diff {
    let mut run: BTreeMap<&Key, usize> = BTreeMap::new();
    for key in keys.iter().flatten() {
        *run.entry(key).or_insert(0) += 1;
    }
    let mut new = Counts::default();
    let mut existing = Counts::default();
    let status = keys
        .iter()
        .map(|key| {
            let Some(key) = key else { return Status::Incomplete };
            if run[key] <= baseline.get(key).copied().unwrap_or(0) {
                existing.add(&key.class);
                Status::Existing
            } else {
                new.add(&key.class);
                Status::New
            }
        })
        .collect();
    let fixed = baseline
        .iter()
        .flat_map(|(key, &b)| {
            let c = run.get(key).copied().unwrap_or(0);
            std::iter::repeat_n(key.clone(), b.saturating_sub(c))
        })
        .collect();
    Diff { status, fixed, new, existing }
}

/// The exit code of a run with `--baseline` (BL-8), from the checker's exit
/// code and the run's error counts.
pub fn exit_code(checker: i32, incomplete: bool, new_errors: usize, errors: usize) -> i32 {
    let completed = checker == 0 || checker == 1;
    if !completed || incomplete {
        return if completed { 2 } else { checker };
    }
    if new_errors > 0 || (checker == 1 && errors == 0) {
        1
    } else {
        0
    }
}

/// The checker's JSON with only new and incomplete results, the exit code of
/// BL-8 and a `baseline` object (BL-9).
pub fn render_json(checker: &serde_json::Value, diff: &Diff, exit_code: i32) -> serde_json::Value {
    let mut out = checker.clone();
    let results: Vec<serde_json::Value> = checker["results"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .zip(&diff.status)
        .filter(|(_, status)| **status != Status::Existing)
        .map(|(result, _)| result.clone())
        .collect();
    out["results"] = results.into();
    out["exit_code"] = exit_code.into();
    out["baseline"] = serde_json::json!({
        "checker_exit_code": checker["exit_code"],
        "new": diff.new,
        "existing": diff.existing,
        "fixed": diff.fixed,
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report;

    fn result(severity: &str, guarantee: &str, line: i64) -> String {
        format!(
            r#"{{"status": "inconsistent", "severity": "{severity}", "source": {{"file": "m.py", "line": 1, "name": "f"}}, "target": {{"file": "models.py", "line": 3, "name": "M.x"}}, "path": ["f", "M.x"], "hop": ["f", "M.x"], "site": {{"file": "m.py", "line": {line}}}, "source_guarantee": "{guarantee}", "target_requirement": "precision ≤ 2", "verification_level": "EXTRACTED", "suggestion": "s"}}"#
        )
    }

    fn output(results: &[String]) -> CheckerOutput {
        report::parse(&format!(
            r#"{{"summary": {{"contracts_checked": 1, "edges_checked": 1, "paths_checked": 1}}, "results": [{}], "exit_code": 1}}"#,
            results.join(",")
        ))
        .unwrap()
    }

    fn key(class: &str, site_text: &str) -> Key {
        Key {
            class: class.to_string(),
            source: "f".to_string(),
            target: "M.x".to_string(),
            hop: vec!["f".to_string(), "M.x".to_string()],
            source_guarantee: "precision ≤ 4".to_string(),
            target_requirement: "precision ≤ 2".to_string(),
            site_file: "m.py".to_string(),
            site_text: Some(site_text.to_string()),
        }
    }

    fn baseline_of(keys: &[Key]) -> BTreeMap<Key, usize> {
        let mut counts = BTreeMap::new();
        for k in keys {
            *counts.entry(k.clone()).or_insert(0) += 1;
        }
        counts
    }

    #[test]
    fn key_collapses_whitespace_and_holds_no_line_number() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("m.py"), "def f(y):\n    x  =\tround(y,  4)   \n\n\n  x =  round(y, 4)\n").unwrap();
        let out = output(&[result("error", "precision ≤ 4", 2), result("error", "precision ≤ 4", 5)]);
        let keys = keys(&out, dir.path()).unwrap();
        assert_eq!(keys[0], Some(key("error", "x = round(y, 4)")));
        assert_eq!(keys[0], keys[1]);
    }

    #[test]
    fn key_class_follows_uc1_and_incomplete_has_no_key() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("m.py"), "a\n").unwrap();
        let incomplete = r#"{"status": "incomplete", "severity": "error", "source": {"file": "", "line": 0, "name": ""}, "target": {"file": "", "line": 0, "name": ""}, "path": [], "hop": [], "site": {"file": "", "line": 0}, "source_guarantee": "", "target_requirement": "", "verification_level": "", "suggestion": "budget"}"#;
        let out = output(&[
            result("warning", "precision (unspecified)", 1),
            result("warning", "precision (dependent)", 1),
            incomplete.to_string(),
        ]);
        let classes: Vec<Option<String>> =
            keys(&out, dir.path()).unwrap().into_iter().map(|k| k.map(|k| k.class)).collect();
        assert_eq!(classes, vec![Some("unverified".to_string()), Some("warning".to_string()), None]);
    }

    #[test]
    fn result_without_a_site_has_null_site_text() {
        let json = result("error", "precision ≤ 4", 0).replace(r#""site": {"file": "m.py", "line": 0}, "#, "");
        let keys = keys(&output(&[json]), Path::new("/nonexistent")).unwrap();
        let k = keys[0].as_ref().unwrap();
        assert_eq!((k.site_file.as_str(), k.site_text.as_deref()), ("", None));
    }

    #[test]
    fn unreadable_or_short_site_file_is_an_error_naming_it() {
        let dir = tempfile::tempdir().unwrap();
        let err = keys(&output(&[result("error", "precision ≤ 4", 1)]), dir.path()).unwrap_err();
        assert!(format!("{err:#}").contains("m.py"), "{err:#}");
        std::fs::write(dir.path().join("m.py"), "one\ntwo\n").unwrap();
        let err = keys(&output(&[result("error", "precision ≤ 4", 3)]), dir.path()).unwrap_err();
        assert!(format!("{err:#}").contains("m.py has no line 3"), "{err:#}");
        let err = keys(&output(&[result("error", "precision ≤ 4", 0)]), dir.path()).unwrap_err();
        assert!(format!("{err:#}").contains("m.py has no line 0"), "{err:#}");
    }

    #[test]
    fn equal_counts_are_existing() {
        let k = key("error", "a");
        let d = diff(&[Some(k.clone()), Some(k.clone())], &baseline_of(&[k.clone(), k]));
        assert_eq!(d.status, vec![Status::Existing, Status::Existing]);
        assert_eq!(d.existing, Counts { errors: 2, warnings: 0, unverified: 0 });
        assert_eq!(d.new, Counts::default());
        assert!(d.fixed.is_empty());
    }

    #[test]
    fn more_results_than_baseline_findings_makes_every_one_new() {
        let k = key("error", "a");
        let w = key("warning", "b");
        let d = diff(&[Some(k.clone()), Some(w.clone()), Some(k.clone())], &baseline_of(&[k, w]));
        assert_eq!(d.status, vec![Status::New, Status::Existing, Status::New]);
        assert_eq!(d.new, Counts { errors: 2, warnings: 0, unverified: 0 });
        assert_eq!(d.existing, Counts { errors: 0, warnings: 1, unverified: 0 });
        assert!(d.fixed.is_empty());
    }

    #[test]
    fn fewer_results_than_baseline_findings_leaves_the_rest_fixed() {
        let k = key("error", "a");
        let u = key("unverified", "c");
        let d = diff(&[Some(k.clone()), None], &baseline_of(&[k.clone(), u.clone(), k.clone(), k.clone()]));
        assert_eq!(d.status, vec![Status::Existing, Status::Incomplete]);
        assert_eq!(d.fixed, vec![k.clone(), k, u]);
    }

    #[test]
    fn exit_code_follows_bl8() {
        // Checker failed or incomplete.
        assert_eq!(exit_code(3, false, 0, 0), 3);
        assert_eq!(exit_code(2, true, 0, 0), 2);
        assert_eq!(exit_code(0, true, 0, 0), 2);
        assert_eq!(exit_code(1, true, 1, 1), 2);
        // A new error.
        assert_eq!(exit_code(1, false, 1, 3), 1);
        assert_eq!(exit_code(0, false, 1, 1), 1);
        // Checker exit 1 with no error at all.
        assert_eq!(exit_code(1, false, 0, 0), 1);
        // Every error is in the baseline.
        assert_eq!(exit_code(1, false, 0, 2), 0);
        assert_eq!(exit_code(0, false, 0, 0), 0);
    }

    #[test]
    fn baseline_file_round_trips_sorted() {
        let keys = vec![Some(key("warning", "b")), None, Some(key("error", "a"))];
        let text = render(&keys);
        let file: BaselineFile = serde_json::from_str(&text).unwrap();
        assert_eq!(file.format, "cgv-baseline/1");
        assert_eq!(file.findings, vec![key("error", "a"), key("warning", "b")]);
        assert_eq!(parse(&text).unwrap(), baseline_of(&[key("error", "a"), key("warning", "b")]));
        assert_eq!(render(&[Some(key("error", "a")), Some(key("warning", "b"))]), text);
    }

    #[test]
    fn malformed_baselines_are_rejected() {
        let good = render(&[Some(key("error", "a"))]);
        assert!(parse(&good).is_ok());
        let wrong_format = good.replace("cgv-baseline/1", "cgv-baseline/2");
        assert!(format!("{:#}", parse(&wrong_format).unwrap_err()).contains("cgv-baseline/2"));
        for field in ["\"site_text\"", "\"hop\"", "\"class\""] {
            let without: serde_json::Value = {
                let mut v: serde_json::Value = serde_json::from_str(&good).unwrap();
                v["findings"][0].as_object_mut().unwrap().remove(field.trim_matches('"'));
                v
            };
            assert!(parse(&without.to_string()).is_err(), "missing {field} accepted");
        }
        let null_text = good.replace("\"site_text\": \"a\"", "\"site_text\": null");
        assert!(parse(&null_text).is_ok());
        assert!(parse("not json").is_err());
        assert!(parse(r#"{"findings": []}"#).is_err());
    }
}
