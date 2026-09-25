//! Human-readable rendering of the Lean checker's JSON output.
//!
//! The Lean checker (`prover/ContractGraph/Main.lean`) always emits JSON on
//! stdout (see `docs/design/system-design.md` section 5.2). This module
//! parses that JSON and renders the text format described in section 5.3.
//! The JSON format itself is untouched by this module and remains the
//! default `contracts check` output.

use serde::Deserialize;

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

impl CheckResult {
    fn is_error(&self) -> bool {
        self.severity == "error" && !self.is_incomplete()
    }

    /// The run stopped before checking every path (e.g. the path budget
    /// was exceeded); `suggestion` says why.
    fn is_incomplete(&self) -> bool {
        self.status == "incomplete"
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
        if !self.is_error() || self.path.len() < 3 || self.hop.len() < 2 {
            return false;
        }
        let first_edge = &self.path[0..2];
        self.hop.as_slice() != first_edge
    }
}

/// Parse the checker's raw JSON stdout into a `CheckerOutput`.
pub fn parse(json: &str) -> Result<CheckerOutput, serde_json::Error> {
    serde_json::from_str(json)
}

/// Render the checker output as human-readable text (section 5.3 of the
/// system design doc). When `no_warnings` is set, warning blocks are
/// omitted from the body, but warnings are still counted in the final
/// `RESULT:` line.
pub fn render_text(output: &CheckerOutput, no_warnings: bool) -> String {
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

    let errors: Vec<&CheckResult> = output.results.iter().filter(|r| r.is_error()).collect();
    let warnings: Vec<&CheckResult> = output
        .results
        .iter()
        .filter(|r| !r.is_error() && !r.is_incomplete())
        .collect();

    for result in output.results.iter().filter(|r| r.is_incomplete()) {
        out.push_str(&format!("\nINCOMPLETE  {}\n", result.suggestion));
    }
    for result in errors.iter().chain(warnings.iter().filter(|_| !no_warnings)) {
        out.push('\n');
        out.push_str(&render_result(result));
    }

    out.push('\n');
    out.push_str(&format!(
        "RESULT: {} error{}, {} warning{}. Exit code {}.\n",
        errors.len(),
        if errors.len() == 1 { "" } else { "s" },
        warnings.len(),
        if warnings.len() == 1 { "" } else { "s" },
        output.exit_code
    ));

    out
}

fn render_result(result: &CheckResult) -> String {
    let label = if result.is_error() { "ERROR" } else { "WARNING" };
    let mut lines = Vec::new();

    lines.push(format!(
        "{label}  {}:{} \u{2192} {}:{}",
        result.source.file, result.source.line, result.target.file, result.target.line
    ));
    lines.push(format!(
        "       {} guarantees {}",
        result.source.name, result.source_guarantee
    ));
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
        let text = render_text(&output, false);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "CONTRACTS CHECKED: 12");
        assert_eq!(lines[1], "EDGES CHECKED: 3");
        assert_eq!(lines[2], "STATES CHECKED: 2");
    }

    #[test]
    fn transitive_error_has_failing_hop_and_note() {
        let output = parse(TRANSITIVE_JSON).unwrap();
        let text = render_text(&output, false);
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
        let text = render_text(&output, false);
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
        let text = render_text(&output, false);
        assert!(!text.contains("Failing hop:"));
        assert!(!text.contains("Note: invisible to pairwise checking."));
        assert!(text.contains("Path: split_energy \u{2192} EnergyRecord.energy"));
    }

    #[test]
    fn no_warnings_flag_hides_warning_blocks_but_keeps_count() {
        let output = parse(TRANSITIVE_JSON).unwrap();
        let text = render_text(&output, true);
        assert!(!text.contains("WARNING"));
        assert!(text.contains("RESULT: 1 error, 1 warning. Exit code 1."));
    }

    #[test]
    fn result_line_pluralizes_correctly() {
        let output = parse(BUG1_JSON).unwrap();
        let text = render_text(&output, false);
        assert!(text.ends_with("RESULT: 1 error, 0 warnings. Exit code 1.\n"));
    }

    #[test]
    fn incomplete_result_is_reported_as_such() {
        let json = r#"{"summary": {"contracts_checked": 5, "edges_checked": 7, "paths_checked": 0}, "results": [{"status": "incomplete", "severity": "error", "source": {"file": "", "line": 0, "name": ""}, "target": {"file": "", "line": 0, "name": ""}, "path": [], "hop": [], "site": {"file": "", "line": 0}, "source_guarantee": "", "target_requirement": "", "verification_level": "", "suggestion": "Path budget exceeded (--max-paths 10)."}], "exit_code": 2}"#;
        let text = render_text(&parse(json).unwrap(), false);
        assert!(text.contains("INCOMPLETE  Path budget exceeded (--max-paths 10)."), "{text}");
        assert!(!text.contains("ERROR"), "{text}");
        assert!(text.ends_with("RESULT: 0 errors, 0 warnings. Exit code 2.\n"), "{text}");
    }

    #[test]
    fn site_is_shown() {
        let json = BUG1_JSON.replace(r#""hop": ["#, r#""site": {"file": "billing/services.py", "line": 7}, "hop": ["#);
        let text = render_text(&parse(&json).unwrap(), false);
        assert!(text.contains("At: billing/services.py:7"), "{text}");
    }

    #[test]
    fn clean_result_has_no_blocks() {
        let output = parse(CLEAN_JSON).unwrap();
        let text = render_text(&output, false);
        assert!(!text.contains("ERROR"));
        assert!(!text.contains("WARNING"));
        assert!(text.contains("RESULT: 0 errors, 0 warnings. Exit code 0."));
    }
}
