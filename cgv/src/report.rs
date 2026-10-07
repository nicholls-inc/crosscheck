//! Human-readable rendering of the Lean checker's JSON output.
//!
//! The Lean checker (`prover/ContractGraph/Main.lean`) always emits JSON on
//! stdout (see `docs/design/system-design.md` section 5.2). This module
//! parses that JSON and renders the text format described in section 5.3.
//! The JSON format itself is untouched by this module and remains the
//! default `contracts check` output.

use serde::Deserialize;
use std::collections::BTreeMap;

use crate::baseline::{Diff, Status};

#[derive(Debug, Deserialize)]
pub struct CheckerOutput {
    #[serde(default)]
    pub summary: Summary,
    #[serde(default)]
    pub results: Vec<CheckResult>,
    pub exit_code: i32,
}

#[derive(Debug, Default, Deserialize)]
pub struct Summary {
    pub contracts_checked: i64,
    pub edges_checked: i64,
    pub paths_checked: i64,
}

#[derive(Debug, Deserialize)]
pub struct NodeRef {
    pub file: String,
    pub line: i64,
    pub name: String,
}

/// Location of the write or call expression of the failing hop.
#[derive(Debug, Deserialize)]
pub struct Site {
    pub file: String,
    pub line: i64,
}

#[derive(Debug, Deserialize)]
pub struct CheckResult {
    pub status: String,
    pub severity: String,
    pub source: NodeRef,
    pub target: NodeRef,
    pub path: Vec<String>,
    pub hop: Vec<String>,
    #[serde(default)]
    pub site: Option<Site>,
    pub source_guarantee: String,
    pub target_requirement: String,
    pub verification_level: String,
    pub suggestion: String,
}

impl CheckerOutput {
    /// Results that are neither errors nor an incomplete run: warnings and
    /// unverified requirements.
    pub fn warning_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| matches!(r.class(), Class::Warning | Class::Unverified { .. }))
            .count()
    }

    fn is_incomplete(&self) -> bool {
        self.results.iter().any(|r| r.class() == Class::Incomplete)
    }
}

/// What a checker result means to a reader of the text report (UC-1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class<'a> {
    /// The run stopped before checking every state; `suggestion` says why.
    Incomplete,
    Error,
    /// The hop's source has no guarantee of `kind`, so the target's `kind`
    /// requirement passes vacuously (`missingPostconditionWarning`).
    Unverified { kind: &'a str },
    /// Any other warning, such as an unresolved dependent bound.
    Warning,
}

impl CheckResult {
    /// The UC-1 class as a baseline key names it, or `None` for an
    /// incomplete run (BL-3).
    pub fn class_name(&self) -> Option<&'static str> {
        match self.class() {
            Class::Incomplete => None,
            Class::Error => Some("error"),
            Class::Warning => Some("warning"),
            Class::Unverified { .. } => Some("unverified"),
        }
    }

    fn class(&self) -> Class<'_> {
        if self.status == "incomplete" {
            return Class::Incomplete;
        }
        if self.severity == "error" {
            return Class::Error;
        }
        if self.suggestion.starts_with("Dependent expression on ") {
            return Class::Warning;
        }
        match self.source_guarantee.strip_suffix(" (unspecified)") {
            Some(kind) if !kind.is_empty() && !kind.contains(' ') => Class::Unverified { kind },
            _ => Class::Warning,
        }
    }

    /// Whether the failing hop should be shown separately from the full
    /// path -- i.e. the hop is a proper sub-edge of a longer path.
    fn hop_differs_from_path(&self) -> bool {
        self.hop != self.path
    }

    /// Whether this result should carry the "invisible to pairwise
    /// checking" note: an error on a path of 3+ nodes whose failing hop is
    /// not the first edge in the path.
    fn is_transitive_only_note(&self) -> bool {
        if self.class() != Class::Error || self.path.len() < 3 || self.hop.len() < 2 {
            return false;
        }
        let first_edge = &self.path[0..2];
        self.hop.as_slice() != first_edge
    }

    /// The file of the write or call site, else the source node's file.
    fn site_file(&self) -> &str {
        match &self.site {
            Some(site) if !site.file.is_empty() => &site.file,
            _ => &self.source.file,
        }
    }
}

/// Which non-error results the text body prints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WarningDisplay {
    /// `--no-warnings`: neither warnings nor unverified requirements.
    Hide,
    /// The default: warnings, but not unverified requirements.
    Default,
    /// `--warnings`: warnings and unverified requirements.
    All,
}

/// Parse the checker's raw JSON stdout into a `CheckerOutput`.
pub fn parse(json: &str) -> Result<CheckerOutput, serde_json::Error> {
    serde_json::from_str(json)
}

/// The top-level module of a path relative to the checked path (UC-7).
pub fn module_of(path: &str) -> String {
    let parts: Vec<&str> = std::path::Path::new(path)
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(p) => p.to_str(),
            _ => None,
        })
        .collect();
    match parts.as_slice() {
        [] => "(unknown)".to_string(),
        [file] => file.strip_suffix(".py").unwrap_or(file).to_string(),
        [first, ..] => first.to_string(),
    }
}

/// Count the edges of the contract database by the module of their site, or
/// of their source node when they have no site (UC-7, UC-8).
pub fn edges_by_module(db: &std::path::Path) -> rusqlite::Result<BTreeMap<String, usize>> {
    let conn = rusqlite::Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut stmt = conn.prepare(
        "SELECT COALESCE(NULLIF(e.site_file, ''), n.source_file, '') \
         FROM edges e LEFT JOIN nodes n ON n.id = e.source_node_id",
    )?;
    let mut counts = BTreeMap::new();
    for file in stmt.query_map([], |row| row.get::<_, String>(0))? {
        *counts.entry(module_of(&file?)).or_insert(0) += 1;
    }
    Ok(counts)
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// Render the checker output as human-readable text (section 5.3 of the
/// system design doc). `edges` counts the edges of the checked graph by
/// module (`edges_by_module`).
pub fn render_text(
    output: &CheckerOutput,
    display: WarningDisplay,
    edges: &BTreeMap<String, usize>,
) -> String {
    render_text_with(output, display, edges, None)
}

/// `render_text`, and with `baseline` (the comparison and the exit code of
/// BL-8) the body shows only incomplete and new results (BL-10).
pub fn render_text_with(
    output: &CheckerOutput,
    display: WarningDisplay,
    edges: &BTreeMap<String, usize>,
    baseline: Option<(&Diff, i32)>,
) -> String {
    let mut out = String::new();

    out.push_str(&format!(
        "CONTRACTS CHECKED: {}\n",
        output.summary.contracts_checked
    ));
    out.push_str(&format!(
        "EDGES CHECKED: {}\n",
        output.summary.edges_checked
    ));
    out.push_str(&format!(
        "STATES CHECKED: {}\n",
        output.summary.paths_checked
    ));

    let shown = |i: usize| baseline.is_none_or(|(diff, _)| diff.status[i] != Status::Existing);
    let of = |want: fn(&Class) -> bool, new_only: bool| -> Vec<&CheckResult> {
        output
            .results
            .iter()
            .enumerate()
            .filter(|(i, r)| want(&r.class()) && (!new_only || shown(*i)))
            .map(|(_, r)| r)
            .collect()
    };
    let incomplete = of(|c| *c == Class::Incomplete, false);
    let errors = of(|c| *c == Class::Error, true);
    let warnings = of(|c| *c == Class::Warning, true);
    let unverified = of(|c| matches!(c, Class::Unverified { .. }), true);
    let all_unverified = of(|c| matches!(c, Class::Unverified { .. }), false);

    for result in &incomplete {
        out.push_str(&format!("\nINCOMPLETE  {}\n", result.suggestion));
    }
    let shown_warnings: &[&CheckResult] = if display == WarningDisplay::Hide { &[] } else { &warnings };
    let shown_unverified: &[&CheckResult] = if display == WarningDisplay::All { &unverified } else { &[] };
    for result in errors.iter().chain(shown_warnings).chain(shown_unverified) {
        out.push('\n');
        out.push_str(&render_result(result));
    }

    if !output.is_incomplete() {
        out.push_str(&render_coverage(&all_unverified, edges, display));
    }

    out.push('\n');
    match baseline {
        None => out.push_str(&format!(
            "RESULT: {}, {}, {} unverified. Exit code {}.\n",
            plural(errors.len(), "error", "errors"),
            plural(warnings.len(), "warning", "warnings"),
            unverified.len(),
            output.exit_code
        )),
        Some((diff, exit_code)) => {
            out.push_str(&format!(
                "BASELINE: {}, {}, {} unverified in the baseline, not shown.\n",
                plural(diff.existing.errors, "error", "errors"),
                plural(diff.existing.warnings, "warning", "warnings"),
                diff.existing.unverified,
            ));
            if !diff.fixed.is_empty() {
                out.push_str(&format!(
                    "FIXED: {} in the baseline not reported by this run:\n",
                    plural(diff.fixed.len(), "finding", "findings")
                ));
                for key in &diff.fixed {
                    let hop_source = key.hop.first().unwrap_or(&key.source);
                    let hop_target = key.hop.last().unwrap_or(&key.target);
                    out.push_str(&format!("  {} {hop_source} \u{2192} {hop_target}", key.class));
                    if let Some(text) = &key.site_text {
                        out.push_str(&format!(" at {}: {text}", key.site_file));
                    }
                    out.push('\n');
                }
            }
            out.push_str(&format!(
                "RESULT: {}, {}, {} new unverified. Exit code {exit_code}.\n",
                plural(errors.len(), "new error", "new errors"),
                plural(warnings.len(), "new warning", "new warnings"),
                unverified.len(),
            ));
        }
    }

    out
}

/// The coverage section (UC-6).
fn render_coverage(
    unverified: &[&CheckResult],
    edges: &BTreeMap<String, usize>,
    display: WarningDisplay,
) -> String {
    let mut by_module: BTreeMap<String, BTreeMap<&str, usize>> = BTreeMap::new();
    for result in unverified {
        if let Class::Unverified { kind } = result.class() {
            *by_module
                .entry(module_of(result.site_file()))
                .or_default()
                .entry(kind)
                .or_insert(0) += 1;
        }
    }
    let modules: std::collections::BTreeSet<&String> = edges.keys().chain(by_module.keys()).collect();
    if modules.is_empty() {
        return String::new();
    }

    let mut out = String::from("\nCOVERAGE BY MODULE\n");
    for module in modules {
        let kinds = by_module.get(module);
        let total: usize = kinds.map_or(0, |k| k.values().sum());
        out.push_str(&format!(
            "  {module}: {} checked, {} unverified",
            plural(edges.get(module).copied().unwrap_or(0), "edge", "edges"),
            plural(total, "requirement", "requirements"),
        ));
        if let Some(kinds) = kinds {
            let list: Vec<String> = kinds.iter().map(|(k, n)| format!("{k} {n}")).collect();
            out.push_str(&format!(" ({})", list.join(", ")));
        }
        out.push('\n');
    }
    if !unverified.is_empty() {
        out.push_str(
            "An unverified requirement is not yet reached: the value's source has no guarantee \
             of the kind the target requires, so the requirement passes vacuously. Open question: \
             which guarantee the extractor could infer for such a value, or which annotation it \
             should ask for.",
        );
        if display != WarningDisplay::All {
            out.push_str(" Pass --warnings to list them.");
        }
        out.push('\n');
    }
    out
}

fn render_result(result: &CheckResult) -> String {
    let class = result.class();
    let label = match class {
        Class::Error => "ERROR",
        Class::Unverified { .. } => "UNVERIFIED",
        Class::Warning | Class::Incomplete => "WARNING",
    };
    let hop_source = result.hop.first().map_or(result.source.name.as_str(), String::as_str);
    let hop_target = result.hop.last().map_or(result.target.name.as_str(), String::as_str);
    let mut lines = Vec::new();

    lines.push(format!(
        "{label}  {}:{} \u{2192} {}:{}",
        result.source.file, result.source.line, result.target.file, result.target.line
    ));
    match class {
        Class::Unverified { kind } => lines.push(format!(
            "       no {kind} guarantee for the value {hop_source} passes to {hop_target}"
        )),
        _ => lines.push(format!(
            "       {} guarantees {}",
            result.source.name, result.source_guarantee
        )),
    }
    lines.push(format!(
        "       {} requires {}",
        result.target.name, result.target_requirement
    ));
    lines.push(format!("       Path: {}", result.path.join(" \u{2192} ")));
    if result.hop_differs_from_path() {
        lines.push(format!(
            "       Failing hop: {}",
            result.hop.join(" \u{2192} ")
        ));
    }
    if let Some(site) = &result.site {
        lines.push(format!("       At: {}:{}", site.file, site.line));
    }
    lines.push(format!(
        "       Path verification level: {}",
        result.verification_level
    ));
    if result.is_transitive_only_note() {
        lines.push("       Note: invisible to pairwise checking.".to_string());
    }
    lines.push(format!("       Suggestion: {}", result.suggestion));

    lines.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRANSITIVE_JSON: &str = r#"{"summary": {"contracts_checked": 12, "edges_checked": 3, "paths_checked": 2}, "results": [{"status": "inconsistent", "severity": "error", "source": {"file": "utils.py", "line": 27, "name": "compute_offpeak"}, "target": {"file": "models.py", "line": 5, "name": "EnergyRecord.energy"}, "path": ["compute_offpeak", "split_energy", "EnergyRecord.energy"], "hop": ["split_energy", "EnergyRecord.energy"], "source_guarantee": "precision ≤ 4", "target_requirement": "precision ≤ 3", "verification_level": "ASSUMED", "suggestion": "Source guarantees ≤ 4, target requires ≤ 3. Either tighten the source or widen the target."}, {"status": "inconsistent", "severity": "warning", "source": {"file": "utils.py", "line": 27, "name": "split_energy"}, "target": {"file": "utils.py", "line": 27, "name": "EnergyRecord.energy"}, "path": ["split_energy", "EnergyRecord.energy"], "hop": ["split_energy", "EnergyRecord.energy"], "source_guarantee": "precision (dependent)", "target_requirement": "precision (dependent)", "verification_level": "EXTRACTED", "suggestion": "Dependent expression on split_energy could not be resolved. Check that upstream postconditions provide the required input bindings."}], "exit_code": 1}"#;

    const BUG1_JSON: &str = r#"{"summary": {"contracts_checked": 2, "edges_checked": 1, "paths_checked": 1}, "results": [{"status": "inconsistent", "severity": "error", "source": {"file": "billing/utils.py", "line": 42, "name": "split_energy"}, "target": {"file": "billing/models.py", "line": 15, "name": "EnergyRecord.energy"}, "path": ["split_energy", "EnergyRecord.energy"], "hop": ["split_energy", "EnergyRecord.energy"], "source_guarantee": "precision ≤ 6", "target_requirement": "precision ≤ 3", "verification_level": "EXTRACTED", "suggestion": "Reduce function output precision to <= 3, or widen model decimal_places to 6."}], "exit_code": 1}"#;

    const CLEAN_JSON: &str = r#"{"summary": {"contracts_checked": 4, "edges_checked": 2, "paths_checked": 1}, "results": [], "exit_code": 0}"#;

    fn no_edges() -> BTreeMap<String, usize> {
        BTreeMap::new()
    }

    #[test]
    fn parses_checker_json() {
        let output = parse(TRANSITIVE_JSON).expect("should parse");
        assert_eq!(output.summary.contracts_checked, 12);
        assert_eq!(output.summary.edges_checked, 3);
        assert_eq!(output.summary.paths_checked, 2);
        assert_eq!(output.results.len(), 2);
        assert_eq!(output.exit_code, 1);
    }

    #[test]
    fn invalid_json_is_rejected() {
        assert!(parse("not json").is_err());
    }

    #[test]
    fn header_reports_counts() {
        let output = parse(TRANSITIVE_JSON).unwrap();
        let text = render_text(&output, WarningDisplay::Default, &no_edges());
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "CONTRACTS CHECKED: 12");
        assert_eq!(lines[1], "EDGES CHECKED: 3");
        assert_eq!(lines[2], "STATES CHECKED: 2");
    }

    #[test]
    fn transitive_error_has_failing_hop_and_note() {
        let output = parse(TRANSITIVE_JSON).unwrap();
        let text = render_text(&output, WarningDisplay::Default, &no_edges());
        assert!(text.contains("ERROR  utils.py:27 \u{2192} models.py:5"));
        assert!(text.contains("compute_offpeak guarantees precision \u{2264} 4"));
        assert!(text.contains("EnergyRecord.energy requires precision \u{2264} 3"));
        assert!(text.contains(
            "Path: compute_offpeak \u{2192} split_energy \u{2192} EnergyRecord.energy"
        ));
        assert!(text.contains("Failing hop: split_energy \u{2192} EnergyRecord.energy"));
        assert!(text.contains("Path verification level: ASSUMED"));
        assert!(text.contains("Note: invisible to pairwise checking."));
    }

    #[test]
    fn warning_has_no_transitive_note() {
        let output = parse(TRANSITIVE_JSON).unwrap();
        let text = render_text(&output, WarningDisplay::Default, &no_edges());
        let warning_block = text
            .split("\n\n")
            .find(|b| b.starts_with("WARNING"))
            .expect("should have a warning block");
        assert!(!warning_block.contains("Note: invisible to pairwise checking."));
        // Single-hop result: hop == path, so no separate "Failing hop" line.
        assert!(!warning_block.contains("Failing hop:"));
    }

    #[test]
    fn single_hop_error_has_no_failing_hop_or_note() {
        let output = parse(BUG1_JSON).unwrap();
        let text = render_text(&output, WarningDisplay::Default, &no_edges());
        assert!(!text.contains("Failing hop:"));
        assert!(!text.contains("Note: invisible to pairwise checking."));
        assert!(text.contains("Path: split_energy \u{2192} EnergyRecord.energy"));
    }

    #[test]
    fn no_warnings_flag_hides_warning_blocks_but_keeps_count() {
        let output = parse(TRANSITIVE_JSON).unwrap();
        let text = render_text(&output, WarningDisplay::Hide, &no_edges());
        assert!(!text.contains("WARNING"));
        assert!(text.contains("RESULT: 1 error, 1 warning, 0 unverified. Exit code 1."));
    }

    #[test]
    fn result_line_pluralizes_correctly() {
        let output = parse(BUG1_JSON).unwrap();
        let text = render_text(&output, WarningDisplay::Default, &no_edges());
        assert!(text.ends_with("RESULT: 1 error, 0 warnings, 0 unverified. Exit code 1.\n"));
    }

    #[test]
    fn incomplete_result_is_reported_as_such() {
        let json = r#"{"summary": {"contracts_checked": 5, "edges_checked": 7, "paths_checked": 0}, "results": [{"status": "incomplete", "severity": "error", "source": {"file": "", "line": 0, "name": ""}, "target": {"file": "", "line": 0, "name": ""}, "path": [], "hop": [], "site": {"file": "", "line": 0}, "source_guarantee": "", "target_requirement": "", "verification_level": "", "suggestion": "Path budget exceeded (--max-paths 10)."}], "exit_code": 2}"#;
        let text = render_text(&parse(json).unwrap(), WarningDisplay::Default, &no_edges());
        assert!(text.contains("INCOMPLETE  Path budget exceeded (--max-paths 10)."), "{text}");
        assert!(!text.contains("ERROR"), "{text}");
        assert!(text.ends_with("RESULT: 0 errors, 0 warnings, 0 unverified. Exit code 2.\n"), "{text}");
    }

    #[test]
    fn site_is_shown() {
        let json = BUG1_JSON.replace(r#""hop": ["#, r#""site": {"file": "billing/services.py", "line": 7}, "hop": ["#);
        let text = render_text(&parse(&json).unwrap(), WarningDisplay::Default, &no_edges());
        assert!(text.contains("At: billing/services.py:7"), "{text}");
    }

    #[test]
    fn clean_result_has_no_blocks() {
        let output = parse(CLEAN_JSON).unwrap();
        let text = render_text(&output, WarningDisplay::Default, &no_edges());
        assert!(!text.contains("ERROR"));
        assert!(!text.contains("WARNING"));
        assert!(text.contains("RESULT: 0 errors, 0 warnings, 0 unverified. Exit code 0."));
    }

    /// One error, one dependent-bound warning and three missing-guarantee
    /// warnings (two in `billing`, one in a top-level file), in the shape
    /// `missingPostconditionWarning` writes them.
    const MIXED_JSON: &str = r#"{"summary": {"contracts_checked": 9, "edges_checked": 6, "paths_checked": 5}, "results": [
      {"status": "inconsistent", "severity": "warning", "source": {"file": "billing/tasks.py", "line": 3, "name": "sync"}, "target": {"file": "billing/models.py", "line": 8, "name": "Invoice.note"}, "path": ["sync", "Invoice.note"], "hop": ["sync", "Invoice.note"], "site": {"file": "billing/tasks.py", "line": 9}, "source_guarantee": "nullability (unspecified)", "target_requirement": "non-null", "verification_level": "EXTRACTED", "suggestion": "'sync' has no nullability postcondition for the value it passes to 'Invoice.note'. Its nullability requirements pass vacuously — this may hide real inconsistencies."},
      {"status": "inconsistent", "severity": "error", "source": {"file": "billing/tasks.py", "line": 20, "name": "bill"}, "target": {"file": "billing/models.py", "line": 5, "name": "Invoice.total"}, "path": ["bill", "Invoice.total"], "hop": ["bill", "Invoice.total"], "site": {"file": "billing/tasks.py", "line": 22}, "source_guarantee": "precision ≤ 4", "target_requirement": "precision ≤ 2", "verification_level": "EXTRACTED", "suggestion": "Source guarantees ≤ 4, target requires ≤ 2. Either tighten the source or widen the target."},
      {"status": "inconsistent", "severity": "warning", "source": {"file": "billing/tasks.py", "line": 3, "name": "sync"}, "target": {"file": "billing/models.py", "line": 5, "name": "Invoice.total"}, "path": ["sync", "Invoice.total"], "hop": ["sync", "Invoice.total"], "site": {"file": "billing/tasks.py", "line": 9}, "source_guarantee": "precision (unspecified)", "target_requirement": "precision ≤ 2", "verification_level": "EXTRACTED", "suggestion": "'sync' has no precision postcondition for the value it passes to 'Invoice.total'. Its precision requirements pass vacuously — this may hide real inconsistencies."},
      {"status": "inconsistent", "severity": "warning", "source": {"file": "jobs.py", "line": 1, "name": "run"}, "target": {"file": "billing/models.py", "line": 8, "name": "Invoice.note"}, "path": ["run", "Invoice.note"], "hop": ["run", "Invoice.note"], "site": {"file": "", "line": 0}, "source_guarantee": "nullability (unspecified)", "target_requirement": "non-null", "verification_level": "EXTRACTED", "suggestion": "'run' has no nullability postcondition for the value it passes to 'Invoice.note'. Its nullability requirements pass vacuously — this may hide real inconsistencies."},
      {"status": "inconsistent", "severity": "warning", "source": {"file": "billing/tasks.py", "line": 30, "name": "split"}, "target": {"file": "billing/models.py", "line": 5, "name": "Invoice.total"}, "path": ["split", "Invoice.total"], "hop": ["split", "Invoice.total"], "site": {"file": "billing/tasks.py", "line": 31}, "source_guarantee": "precision (dependent)", "target_requirement": "precision ≤ 2", "verification_level": "EXTRACTED", "suggestion": "Dependent expression on split could not be resolved. Check that upstream postconditions provide the required input bindings."}
    ], "exit_code": 1}"#;

    fn mixed_edges() -> BTreeMap<String, usize> {
        BTreeMap::from([("billing".to_string(), 5), ("jobs".to_string(), 1)])
    }

    fn mixed(display: WarningDisplay) -> String {
        render_text(&parse(MIXED_JSON).unwrap(), display, &mixed_edges())
    }

    #[test]
    fn classes_follow_uc1() {
        let output = parse(MIXED_JSON).unwrap();
        let classes: Vec<Class> = output.results.iter().map(|r| r.class()).collect();
        assert_eq!(
            classes,
            vec![
                Class::Unverified { kind: "nullability" },
                Class::Error,
                Class::Unverified { kind: "precision" },
                Class::Unverified { kind: "nullability" },
                Class::Warning,
            ]
        );
    }

    #[test]
    fn unrecognised_warning_is_shown_not_hidden() {
        let json = MIXED_JSON.replace("\"precision (unspecified)\"", "\"precision (no idea)\"");
        let text = render_text(&parse(&json).unwrap(), WarningDisplay::Default, &mixed_edges());
        assert!(text.contains("WARNING  billing/tasks.py:3 \u{2192} billing/models.py:5"), "{text}");
        assert!(text.ends_with("RESULT: 1 error, 2 warnings, 2 unverified. Exit code 1.\n"), "{text}");
    }

    #[test]
    fn default_hides_unverified_but_counts_them() {
        let text = mixed(WarningDisplay::Default);
        assert!(!text.contains("UNVERIFIED  "), "{text}");
        assert!(!text.contains("(unspecified)"), "{text}");
        assert!(text.contains("ERROR  billing/tasks.py:20"), "{text}");
        assert!(text.contains("WARNING  billing/tasks.py:30"), "{text}");
        assert!(text.ends_with("RESULT: 1 error, 1 warning, 3 unverified. Exit code 1.\n"), "{text}");
    }

    #[test]
    fn coverage_counts_edges_and_unverified_kinds_by_module() {
        let text = mixed(WarningDisplay::Default);
        let coverage: Vec<&str> = text
            .lines()
            .skip_while(|l| *l != "COVERAGE BY MODULE")
            .take(4)
            .collect();
        assert_eq!(
            coverage,
            vec![
                "COVERAGE BY MODULE",
                "  billing: 5 edges checked, 2 requirements unverified (nullability 1, precision 1)",
                "  jobs: 1 edge checked, 1 requirement unverified (nullability 1)",
                "An unverified requirement is not yet reached: the value's source has no guarantee \
                 of the kind the target requires, so the requirement passes vacuously. Open question: \
                 which guarantee the extractor could infer for such a value, or which annotation it \
                 should ask for. Pass --warnings to list them.",
            ],
            "{text}"
        );
    }

    #[test]
    fn coverage_lists_a_module_with_unverified_requirements_but_no_edges() {
        let edges = BTreeMap::from([("billing".to_string(), 5)]);
        let text = render_text(&parse(MIXED_JSON).unwrap(), WarningDisplay::Default, &edges);
        assert!(text.contains("\n  jobs: 0 edges checked, 1 requirement unverified (nullability 1)\n"), "{text}");
    }

    #[test]
    fn warnings_flag_lists_unverified_blocks() {
        let text = mixed(WarningDisplay::All);
        let block = text
            .split("\n\n")
            .find(|b| b.starts_with("UNVERIFIED  billing/tasks.py:3 \u{2192} billing/models.py:5"))
            .expect("an UNVERIFIED block for the precision requirement");
        assert_eq!(
            block.lines().nth(1),
            Some("       no precision guarantee for the value sync passes to Invoice.total")
        );
        assert!(!block.contains("guarantees"), "{block}");
        assert_eq!(text.matches("UNVERIFIED  ").count(), 3, "{text}");
        assert!(text.contains("WARNING  billing/tasks.py:30"), "{text}");
        assert!(!text.contains("Pass --warnings"), "{text}");
        assert!(text.ends_with("RESULT: 1 error, 1 warning, 3 unverified. Exit code 1.\n"), "{text}");
    }

    #[test]
    fn unverified_blocks_follow_errors_and_warnings() {
        let text = mixed(WarningDisplay::All);
        let error = text.find("ERROR  ").unwrap();
        let warning = text.find("WARNING  ").unwrap();
        let unverified = text.find("UNVERIFIED  ").unwrap();
        assert!(error < warning && warning < unverified, "{text}");
    }

    #[test]
    fn no_warnings_flag_hides_warnings_and_unverified() {
        let text = mixed(WarningDisplay::Hide);
        assert!(!text.contains("WARNING  "), "{text}");
        assert!(!text.contains("UNVERIFIED  "), "{text}");
        assert!(text.contains("COVERAGE BY MODULE"), "{text}");
        assert!(text.ends_with("RESULT: 1 error, 1 warning, 3 unverified. Exit code 1.\n"), "{text}");
    }

    #[test]
    fn incomplete_run_has_no_coverage() {
        let json = r#"{"summary": {"contracts_checked": 5, "edges_checked": 7, "paths_checked": 0}, "results": [{"status": "incomplete", "severity": "error", "source": {"file": "", "line": 0, "name": ""}, "target": {"file": "", "line": 0, "name": ""}, "path": [], "hop": [], "site": {"file": "", "line": 0}, "source_guarantee": "", "target_requirement": "", "verification_level": "", "suggestion": "Path budget exceeded (--max-paths 10)."}], "exit_code": 2}"#;
        let text = render_text(&parse(json).unwrap(), WarningDisplay::Default, &mixed_edges());
        assert!(!text.contains("COVERAGE"), "{text}");
    }

    #[test]
    fn clean_run_coverage_has_no_note() {
        let text = render_text(&parse(CLEAN_JSON).unwrap(), WarningDisplay::Default, &mixed_edges());
        assert!(text.contains("COVERAGE BY MODULE\n  billing: 5 edges checked, 0 requirements unverified\n  jobs: 1 edge checked, 0 requirements unverified\n\nRESULT"), "{text}");
        assert!(!text.contains("not yet reached"), "{text}");
    }

    #[test]
    fn warning_count_includes_unverified() {
        assert_eq!(parse(MIXED_JSON).unwrap().warning_count(), 4);
    }

    #[test]
    fn module_of_takes_the_first_component() {
        assert_eq!(module_of("billing/tasks.py"), "billing");
        assert_eq!(module_of("./billing/sub/tasks.py"), "billing");
        assert_eq!(module_of("jobs.py"), "jobs");
        assert_eq!(module_of("README"), "README");
        assert_eq!(module_of(""), "(unknown)");
    }

    #[test]
    fn dependent_bound_warning_is_never_unverified() {
        let json = MIXED_JSON.replace("\"precision (dependent)\"", "\"nullability (unspecified)\"");
        let output = parse(&json).unwrap();
        assert_eq!(output.results[4].class(), Class::Warning);
    }

    #[test]
    fn edges_by_module_uses_the_site_else_the_source_node() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db = tmp.path().join("c.sqlite");
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch(
            "CREATE TABLE nodes (id INTEGER PRIMARY KEY, source_file TEXT NOT NULL);
             CREATE TABLE edges (id INTEGER PRIMARY KEY, source_node_id INTEGER NOT NULL, site_file TEXT);
             INSERT INTO nodes VALUES (1, 'billing/tasks.py'), (2, 'jobs.py');
             INSERT INTO edges VALUES (1, 1, 'shop/views.py'), (2, 1, NULL), (3, 1, ''), (4, 2, NULL), (5, 9, NULL);",
        )
        .unwrap();
        drop(conn);
        assert_eq!(
            edges_by_module(&db).unwrap(),
            BTreeMap::from([
                ("(unknown)".to_string(), 1),
                ("billing".to_string(), 2),
                ("jobs".to_string(), 1),
                ("shop".to_string(), 1),
            ])
        );
    }

    #[test]
    fn guarantee_kind_with_a_space_is_a_warning() {
        let json = MIXED_JSON.replace("\"precision (unspecified)\"", "\"precision bound (unspecified)\"");
        assert_eq!(parse(&json).unwrap().results[2].class(), Class::Warning);
    }

    #[test]
    fn unverified_requirement_belongs_to_the_module_of_its_site() {
        let json = MIXED_JSON.replacen(
            "\"site\": {\"file\": \"billing/tasks.py\", \"line\": 9}",
            "\"site\": {\"file\": \"shop/views.py\", \"line\": 9}",
            1,
        );
        let text = render_text(&parse(&json).unwrap(), WarningDisplay::Default, &mixed_edges());
        assert!(text.contains("\n  billing: 5 edges checked, 1 requirement unverified (precision 1)\n"), "{text}");
        assert!(text.contains("\n  shop: 0 edges checked, 1 requirement unverified (nullability 1)\n"), "{text}");
    }
}
