package main

import (
	"errors"
	"io/fs"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// writeTree materialises a map of repo-relative paths to file contents under a
// fresh temp dir and returns the root. Parent dirs are created as needed.
func writeTree(t *testing.T, files map[string]string) string {
	t.Helper()
	root := t.TempDir()
	for rel, content := range files {
		p := filepath.Join(root, rel)
		if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
			t.Fatalf("mkdir %s: %v", filepath.Dir(p), err)
		}
		if err := os.WriteFile(p, []byte(content), 0o644); err != nil {
			t.Fatalf("write %s: %v", p, err)
		}
	}
	return root
}

func hasMatch(items []string, substr string) bool {
	for _, s := range items {
		if strings.Contains(s, substr) {
			return true
		}
	}
	return false
}

func TestParseFrontmatter(t *testing.T) {
	tests := []struct {
		name     string
		content  string
		wantName string // expected value of "name" key, "" if absent
		wantDesc bool   // whether "description" key is present
		wantKeys int
	}{
		{
			name: "present_full",
			content: "---\nname: reason\ndescription: does things\n---\n# body\n" +
				"prose here that is long enough to not be empty",
			wantName: "reason",
			wantDesc: true,
			wantKeys: 2,
		},
		{
			name:     "absent_no_leading_marker",
			content:  "# /assurance-probe\n\n**Layer**: 4\nlots of prose follows here",
			wantName: "",
			wantDesc: false,
			wantKeys: 0,
		},
		{
			name: "folded_scalar",
			content: "---\nname: drt-oracle\ndescription: >-\n  A folded scalar value\n" +
				"  spanning lines.\nargument-hint: \"[x]\"\n---\nbody text long enough",
			wantName: "drt-oracle",
			wantDesc: true,
			wantKeys: 3, // name, description, argument-hint
		},
		{
			name:     "no_closing_marker",
			content:  "---\nname: broken\ndescription: x\nstill no closing fence",
			wantName: "",
			wantDesc: false,
			wantKeys: 0,
		},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			fm, body := parseFrontmatter(tc.content)
			if body != tc.content {
				t.Errorf("body mutated; want original content returned verbatim")
			}
			if got := fm["name"]; got != tc.wantName {
				t.Errorf("name = %q, want %q", got, tc.wantName)
			}
			if _, ok := fm["description"]; ok != tc.wantDesc {
				t.Errorf("description present = %v, want %v", ok, tc.wantDesc)
			}
			if len(fm) != tc.wantKeys {
				t.Errorf("key count = %d, want %d (got %v)", len(fm), tc.wantKeys, fm)
			}
		})
	}
}

func TestReferencedTokens(t *testing.T) {
	doc := "see `/reason` and `/drt-oracle`, also use /crosscheck:lean-spec here. " +
		"`not-a-token` lacks the slash; `/reason` repeats."
	got := referencedTokens(doc)
	want := []string{"drt-oracle", "lean-spec", "reason"}
	if strings.Join(got, ",") != strings.Join(want, ",") {
		t.Errorf("referencedTokens = %v, want %v", got, want)
	}
}

func TestDocumented(t *testing.T) {
	doc := "intro `/reason` mid, and `byfuglien` agent, run /trace-execution now, " +
		"plus crosscheck:lean-impl invocation."
	tests := []struct {
		name string
		want bool
	}{
		{"reason", true},          // `/reason`
		{"byfuglien", true},       // `byfuglien`
		{"trace-execution", true}, // /trace-execution<space>
		{"lean-impl", true},       // crosscheck:lean-impl
		{"journal-context", false},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			if got := documented(tc.name, doc); got != tc.want {
				t.Errorf("documented(%q) = %v, want %v", tc.name, got, tc.want)
			}
		})
	}
}

// baseTree returns a minimal-but-valid plugin tree: one well-formed skill, one
// well-formed agent, both documented, plus an empty ledger. Callers mutate it.
func baseTree() map[string]string {
	return map[string]string{
		"skills/reason/SKILL.md": "---\nname: reason\nadd-mode: bootstrap\ndescription: reasons about code\n---\n" +
			"# /reason\n\nA skill body long enough to clear the empty threshold easily.",
		"agents/byfuglien.md": "---\nname: byfuglien\nadd-mode: bootstrap\ndescription: orchestrates verification\n---\n" +
			"# Byfuglien\n\nbody text.",
		"README.md":                  "Crosscheck ships `/reason` and the `byfuglien` orchestrator.",
		"conformance/claims.json":    `{"version":1,"narrative_claims":[]}`,
		".claude-plugin/plugin.json": `{"name":"crosscheck"}`,
	}
}

func TestPhantomDetection(t *testing.T) {
	files := baseTree()
	// Reference a skill that does not exist on disk.
	files["README.md"] += " It also offers `/ghost-skill` which is not implemented."
	r := analyze(writeTree(t, files))

	if !hasMatch(r.errors, "[phantom]") || !hasMatch(r.errors, "ghost-skill") {
		t.Errorf("expected phantom error for ghost-skill, got errors: %v", r.errors)
	}
	// The real artifacts must not produce phantom errors.
	if hasMatch(r.errors, "reason") || hasMatch(r.errors, "byfuglien") {
		t.Errorf("unexpected phantom error for real artifacts: %v", r.errors)
	}
}

func TestRoutingIntegrity(t *testing.T) {
	files := baseTree()
	// An agent that routes to a skill which does not exist on disk.
	files["agents/router.md"] = "---\nname: router\ndescription: routes work\n---\n" +
		"# Router\n\nFor proofs, hand to `/reason`. For specs, run `/crosscheck:ghost-route` next."
	files["README.md"] += " The `router` agent coordinates the chain."
	r := analyze(writeTree(t, files))

	if !hasMatch(r.errors, "[routing]") || !hasMatch(r.errors, "ghost-route") {
		t.Errorf("expected routing error for ghost-route, got errors: %v", r.errors)
	}
	// A real skill the agent routes to must not be flagged.
	if hasMatch(r.errors, "routes to '/reason'") {
		t.Errorf("valid routing target '/reason' must not be flagged: %v", r.errors)
	}
}

// TestRoutingIgnoresFrontmatter pins the AUTO 5 fix: a `/crosscheck:x` token in
// an agent's frontmatter `description:` is documentation, not a routing edge, so
// it must NOT raise a routing error. Only the agent body is scanned.
func TestRoutingIgnoresFrontmatter(t *testing.T) {
	files := baseTree()
	// The phantom token lives ONLY in the frontmatter description; the body
	// routes to nothing unresolved.
	files["agents/describer.md"] = "---\nname: describer\n" +
		"description: orchestrates the chain; can invoke /crosscheck:phantom-fm style skills\n---\n" +
		"# Describer\n\nFor proofs, hand to `/reason`."
	files["README.md"] += " The `describer` agent coordinates the chain."
	r := analyze(writeTree(t, files))

	if hasMatch(r.errors, "phantom-fm") {
		t.Errorf("a /crosscheck:x token in frontmatter must not raise a routing error: %v", r.errors)
	}
	// Sanity: the same token in the *body* would still be caught.
	files["agents/describer.md"] = "---\nname: describer\ndescription: orchestrates the chain\n---\n" +
		"# Describer\n\nFor specs, run `/crosscheck:phantom-body` next."
	r = analyze(writeTree(t, files))
	if !hasMatch(r.errors, "[routing]") || !hasMatch(r.errors, "phantom-body") {
		t.Errorf("a phantom routing token in the body must still be caught: %v", r.errors)
	}
}

// TestStripFrontmatter pins the body-extraction helper that AUTO 5 relies on.
func TestStripFrontmatter(t *testing.T) {
	tests := []struct {
		name    string
		content string
		want    string
	}{
		{
			name:    "strips_block",
			content: "---\nname: x\ndescription: d\n---\nbody line one\nbody line two",
			want:    "body line one\nbody line two",
		},
		{
			name:    "no_frontmatter_returned_verbatim",
			content: "# Heading\n\nplain body, no frontmatter",
			want:    "# Heading\n\nplain body, no frontmatter",
		},
		{
			name:    "unterminated_returned_verbatim",
			content: "---\nname: broken\nno closing fence",
			want:    "---\nname: broken\nno closing fence",
		},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			if got := stripFrontmatter(tc.content); got != tc.want {
				t.Errorf("stripFrontmatter() = %q, want %q", got, tc.want)
			}
		})
	}
}

func TestOrphanDetection(t *testing.T) {
	files := baseTree()
	// Add a skill that no doc mentions.
	files["skills/lonely/SKILL.md"] = "---\nname: lonely\nadd-mode: bootstrap\ndescription: undocumented\n---\n" +
		"# /lonely\n\nbody text long enough to not be empty at all."
	r := analyze(writeTree(t, files))

	if !hasMatch(r.warnings, "[orphan]") || !hasMatch(r.warnings, "lonely") {
		t.Errorf("expected orphan warning for 'lonely', got warnings: %v", r.warnings)
	}
	// Orphan is a WARNING, never an ERROR.
	if hasMatch(r.errors, "lonely") {
		t.Errorf("orphan must not be an error: %v", r.errors)
	}
}

func TestStructuralMissingFrontmatter(t *testing.T) {
	files := baseTree()
	// This mirrors the real assurance-probe defect: a SKILL.md that opens with a
	// prose header instead of YAML frontmatter cannot register as a skill.
	files["skills/probe/SKILL.md"] = "# /probe\n\n**Layer**: 4\n\nprose body, no frontmatter at all here."
	r := analyze(writeTree(t, files))

	if !hasMatch(r.errors, "[structural]") || !hasMatch(r.errors, "probe") ||
		!hasMatch(r.errors, "missing frontmatter keys ['description', 'name']") {
		t.Errorf("expected structural missing-frontmatter error for 'probe', got: %v", r.errors)
	}
}

func TestStructuralNameMismatch(t *testing.T) {
	files := baseTree()
	files["skills/widget/SKILL.md"] = "---\nname: gadget\ndescription: mislabelled\n---\n" +
		"# /widget\n\nbody text long enough to not be empty here."
	r := analyze(writeTree(t, files))
	if !hasMatch(r.errors, "skill dir 'widget' != frontmatter name 'gadget'") {
		t.Errorf("expected dir/name mismatch error, got: %v", r.errors)
	}
}

func TestStructuralEmptySkill(t *testing.T) {
	files := baseTree()
	files["skills/tiny/SKILL.md"] = "---\nname: tiny\ndescription: d\n---\n"
	r := analyze(writeTree(t, files))
	if !hasMatch(r.errors, "skill 'tiny': SKILL.md is effectively empty") {
		t.Errorf("expected empty-skill error, got: %v", r.errors)
	}
}

func TestModeTagCoverage(t *testing.T) {
	// A module with no add-mode tag is an ERROR (AUTO 6).
	files := baseTree()
	files["skills/untagged/SKILL.md"] = "---\nname: untagged\ndescription: missing its mode\n---\n" +
		"# /untagged\n\nbody text long enough to not be empty at all."
	files["README.md"] += " It also ships `/untagged`."
	r := analyze(writeTree(t, files))
	if !hasMatch(r.errors, "[mode]") || !hasMatch(r.errors, "untagged") {
		t.Errorf("expected [mode] error for the untagged skill, got: %v", r.errors)
	}
	// An invalid mode value is also an ERROR.
	files["skills/untagged/SKILL.md"] = "---\nname: untagged\nadd-mode: legacy\ndescription: bad mode\n---\n" +
		"# /untagged\n\nbody text long enough to not be empty at all."
	r = analyze(writeTree(t, files))
	if !hasMatch(r.errors, "[mode]") || !hasMatch(r.errors, "untagged") {
		t.Errorf("expected [mode] error for invalid add-mode value, got: %v", r.errors)
	}
	// `transitional` is a repo-level mode, NEVER a per-module tag
	// (operating-modes.md + ADR-001). A module tagged `transitional` is the most
	// plausible copy-paste error and MUST be rejected (#232 blocker).
	files["skills/untagged/SKILL.md"] = "---\nname: untagged\nadd-mode: transitional\ndescription: repo-level mode misapplied to a module\n---\n" +
		"# /untagged\n\nbody text long enough to not be empty at all."
	r = analyze(writeTree(t, files))
	if !hasMatch(r.errors, "[mode]") || !hasMatch(r.errors, "untagged") {
		t.Errorf("expected [mode] error for transitional add-mode on a module, got: %v", r.errors)
	}
	// baseTree's tagged modules must NOT trip the check.
	clean := analyze(writeTree(t, baseTree()))
	if hasMatch(clean.errors, "[mode]") {
		t.Errorf("tagged baseTree modules must not produce [mode] errors: %v", clean.errors)
	}
}

func boolp(b bool) *bool { return &b }

func TestPresentArtifactLedger(t *testing.T) {
	tests := []struct {
		name      string
		path      string
		expect    *bool
		create    bool // whether to create the path under root
		wantError bool
	}{
		{"absent_expected_absent", "agents/lowry.md", boolp(false), false, false},
		{"present_expected_absent", "agents/here.md", boolp(false), true, true},
		{"present_expected_present", "agents/here.md", boolp(true), true, false},
		{"absent_expected_present", "agents/gone.md", boolp(true), false, true},
		{"absent_default_present", "agents/gone.md", nil, false, true}, // default expect_present=true
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			files := baseTree()
			if tc.create {
				files[tc.path] = "placeholder"
			}
			expectField := "true"
			if tc.expect != nil && !*tc.expect {
				expectField = "false"
			}
			ledgerJSON := `{"version":1,"narrative_claims":[{"id":"C","source":"s","claim":"c","reality":"r",` +
				`"status":"known-gap","check":{"type":"present_artifact","path":"` + tc.path + `"`
			if tc.expect != nil {
				ledgerJSON += `,"expect_present":` + expectField
			}
			ledgerJSON += `}}]}`
			files["conformance/claims.json"] = ledgerJSON

			r := analyze(writeTree(t, files))
			gotError := hasMatch(r.errors, "[ledger] claim C auto-check failed")
			if gotError != tc.wantError {
				t.Errorf("ledger error = %v, want %v (errors: %v)", gotError, tc.wantError, r.errors)
			}
		})
	}
}

func TestUnreviewedLedgerFails(t *testing.T) {
	files := baseTree()
	files["conformance/claims.json"] = `{"version":1,"narrative_claims":[` +
		`{"id":"C-NEW","source":"s","claim":"c","reality":"r","status":"unreviewed","check":{"type":"manual"}}]}`
	r := analyze(writeTree(t, files))
	if !hasMatch(r.errors, "[ledger] claim C-NEW is UNREVIEWED") {
		t.Errorf("expected UNREVIEWED ledger error, got: %v", r.errors)
	}
}

func TestKnownGapNeedsTracking(t *testing.T) {
	// A known-gap with no tracked_in link fails CI.
	files := baseTree()
	files["conformance/claims.json"] = `{"version":1,"narrative_claims":[` +
		`{"id":"C-GAP","source":"s","claim":"c","reality":"r","status":"known-gap","check":{"type":"manual"}}]}`
	r := analyze(writeTree(t, files))
	if !hasMatch(r.errors, "claim C-GAP is a known-gap with no tracked_in link") {
		t.Errorf("expected known-gap-without-tracking error, got: %v", r.errors)
	}

	// The same gap with a tracked_in link is clean.
	files["conformance/claims.json"] = `{"version":1,"narrative_claims":[` +
		`{"id":"C-GAP","source":"s","claim":"c","reality":"r","status":"known-gap",` +
		`"tracked_in":"https://github.com/nicholls-inc/claude-code-marketplace/issues/217","check":{"type":"manual"}}]}`
	r = analyze(writeTree(t, files))
	if hasMatch(r.errors, "no tracked_in link") {
		t.Errorf("known-gap with tracking should not error, got: %v", r.errors)
	}
}

func TestLedgerStatusAllowlist(t *testing.T) {
	tests := []struct {
		name      string
		status    string
		wantError string
	}{
		{"typo", `"status":"reviewed-disclsed",`, `[ledger] claim C has unknown status "reviewed-disclsed"`},
		{"empty", `"status":"",`, `[ledger] claim C has unknown status ""`},
		{"missing", ``, `[ledger] cannot parse conformance/claims.json: narrative_claims[0].status is missing`},
		{"padded", `"status":" reviewed-accurate",`, `[ledger] claim C has unknown status " reviewed-accurate"`},
		{"unreviewed", `"status":"unreviewed",`, ""},
		{"known_gap", `"status":"known-gap",`, ""},
		{"reviewed_disclosed", `"status":"reviewed-disclosed",`, ""},
		{"reviewed_accurate", `"status":"reviewed-accurate",`, ""},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			files := baseTree()
			files["conformance/claims.json"] = `{"version":1,"narrative_claims":[{"id":"C","source":"s","claim":"c","reality":"r",` +
				tc.status + `"tracked_in":"#25","check":{"type":"manual"}}]}`
			r := analyze(writeTree(t, files))
			if tc.wantError != "" {
				if !hasMatch(r.errors, tc.wantError) {
					t.Errorf("want error %q, got: %v", tc.wantError, r.errors)
				}
				return
			}
			if hasMatch(r.errors, "unknown status") {
				t.Errorf("status %s must be accepted, got: %v", tc.status, r.errors)
			}
		})
	}
}

func TestLedgerLoad(t *testing.T) {
	const readErr = "[ledger] cannot read conformance/claims.json: "
	const parseErr = "[ledger] cannot parse conformance/claims.json: "
	writeLedger := func(content string) func(t *testing.T, path string) {
		return func(t *testing.T, path string) {
			if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
				t.Fatal(err)
			}
		}
	}
	// okClaim meets every LL-9 rule. ledger wraps claims in a valid top level,
	// and claimWith builds a claim from its keys, so each case changes one thing.
	const okClaim = `{"id":"C1","source":"s","claim":"c","reality":"r","status":"reviewed-accurate","check":{"type":"manual"}}`
	ledger := func(claims ...string) func(t *testing.T, path string) {
		return writeLedger(`{"version":1,"narrative_claims":[` + strings.Join(claims, ",") + `]}`)
	}
	claimWith := func(keys string) string {
		return `{` + keys + `}`
	}
	const base = `"id":"C1","source":"s","claim":"c","reality":"r","status":"reviewed-accurate"`
	symlink := func(target string) func(t *testing.T, path string) {
		return func(t *testing.T, path string) {
			if err := os.Symlink(target, path); err != nil {
				t.Fatal(err)
			}
		}
	}
	tests := []struct {
		name       string
		setup      func(t *testing.T, path string)
		wantPrefix string
		wantClaims int
	}{
		{"missing", func(*testing.T, string) {}, "", 0},
		{"no_conformance_dir", func(t *testing.T, path string) {
			if err := os.RemoveAll(filepath.Dir(path)); err != nil {
				t.Fatal(err)
			}
		}, "", 0},
		{"directory", func(t *testing.T, path string) {
			if err := os.Mkdir(path, 0o755); err != nil {
				t.Fatal(err)
			}
		}, readErr, 0},
		{"no_permission", func(t *testing.T, path string) {
			if os.Geteuid() == 0 {
				t.Skip("root reads a mode-000 file")
			}
			writeLedger(`{"version":1,"narrative_claims":[]}`)(t, path)
			if err := os.Chmod(path, 0o000); err != nil {
				t.Fatal(err)
			}
		}, readErr, 0},
		{"truncated", writeLedger(`{"version":1,"narrative_claims":[`), parseErr + "the ledger ends before its top-level value is complete", 0},
		{"empty", writeLedger(``), parseErr + "the ledger is empty", 0},
		{"whitespace_only", writeLedger(" \t\r\n"), parseErr + "the ledger is empty", 0},
		{"not_json", writeLedger(`{"version":1,,"narrative_claims":[]}`), parseErr + "the ledger is not valid JSON at byte 13", 0},
		{"invalid_utf8_before_syntax_error", ledger(`{"id":"C1",,"source":"` + "\xff" + `"}`), parseErr + "the ledger is not valid UTF-8 at byte 55", 0},
		{"invalid_utf8_before_trailing_data", writeLedger(`{"version":1,"narrative_claims":[]} ` + "\xff"), parseErr + "the ledger is not valid UTF-8 at byte 36", 0},
		{"byte_order_mark", writeLedger("\xef\xbb\xbf" + `{"version":1,"narrative_claims":[]}`), parseErr + "the ledger is not valid JSON at byte 0", 0},
		{"invalid_utf8", ledger(`{"id":"C1","source":"` + "\xff\xfe" + `","claim":"c","reality":"r","status":"reviewed-accurate","check":{"type":"manual"}}`), parseErr + "the ledger is not valid UTF-8 at byte 54", 0},
		{"trailing_whitespace", writeLedger(`{"version":1,"narrative_claims":[]}` + "\n\t\r "), "", 0},
		{"trailing_form_feed", writeLedger(`{"version":1,"narrative_claims":[]}` + "\n\f"), parseErr + "the ledger has data after its top-level value at byte 36", 0},
		{"claims_not_array", writeLedger(`{"version":1,"narrative_claims":{}}`), parseErr + "narrative_claims must be an array", 0},
		{"claims_string", writeLedger(`{"version":1,"narrative_claims":"[]"}`), parseErr + "narrative_claims must be an array", 0},
		// A ledger that breaks the schema is rejected before json.Unmarshal runs,
		// so a first claim that would decode cannot leak out ahead of a second
		// claim with a type error: the schema error names the fault and the
		// ledger stays empty.
		{"partial_decode", writeLedger(`{"version":1,"narrative_claims":[{"id":"C1","source":"s","claim":"c","reality":"r","status":"reviewed-accurate","check":{"type":"manual"}},{"id":5,"source":"s","claim":"c","reality":"r","status":"reviewed-accurate","check":{"type":"manual"}}]}`), parseErr + "narrative_claims[1].id must be a non-blank string", 0},
		{"dangling_symlink", symlink("missing.json"), readErr, 0},
		{"dangling_symlink_chain", func(t *testing.T, path string) {
			symlink("missing.json")(t, filepath.Join(filepath.Dir(path), "hop.json"))
			symlink("hop.json")(t, path)
		}, readErr, 0},
		{"dangling_conformance_dir", func(t *testing.T, path string) {
			dir := filepath.Dir(path)
			if err := os.RemoveAll(dir); err != nil {
				t.Fatal(err)
			}
			symlink("missing-dir")(t, dir)
		}, readErr, 0},
		{"conformance_dir_symlink_no_ledger", func(t *testing.T, path string) {
			dir := filepath.Dir(path)
			if err := os.Rename(dir, dir+"-real"); err != nil {
				t.Fatal(err)
			}
			symlink(filepath.Base(dir)+"-real")(t, dir)
		}, "", 0},
		{"symlink_to_ledger", func(t *testing.T, path string) {
			ledger(okClaim)(t, filepath.Join(filepath.Dir(path), "real.json"))
			symlink("real.json")(t, path)
		}, "", 1},
		{"top_null", writeLedger(`null`), parseErr + "the ledger is null", 0},
		{"top_array", writeLedger(`[]`), parseErr + "the ledger is not an object", 0},
		{"top_empty_object", writeLedger(`{}`), parseErr + "narrative_claims is missing", 0},
		{"claims_missing", writeLedger(`{"version":1}`), parseErr + "narrative_claims is missing", 0},
		{"claims_null", writeLedger(`{"version":1,"narrative_claims":null}`), parseErr + "narrative_claims is null", 0},
		{"claim_null", ledger(okClaim, `null`), parseErr + "narrative_claims[1] is null", 0},
		{"check_null", ledger(claimWith(base + `,"check":null`)), parseErr + "narrative_claims[0].check is null", 0},
		{"check_not_object", ledger(claimWith(base + `,"check":"manual"`)), parseErr + "narrative_claims[0].check is not an object", 0},
		{"unknown_top_key", writeLedger(`{"version":1,"narrative_claims":[],"k5":1,"k9":1,"k3":1,"k8":1,"k1":1,"k7":1,"k2":1,"k6":1,"k4":1}`), parseErr + `the ledger has unknown key "k1"`, 0},
		{"unknown_claim_key", ledger(claimWith(base + `,"tracked-in":"#1","check":{"type":"manual"}`)), parseErr + `narrative_claims[0] has unknown key "tracked-in"`, 0},
		{"unknown_check_key", ledger(claimWith(base + `,"check":{"type":"present_artifact","path":"README.md","expect-present":false}`)), parseErr + `narrative_claims[0].check has unknown key "expect-present" for type "present_artifact"`, 0},
		{"key_case_mismatch", writeLedger(`{"Narrative_Claims":[]}`), parseErr + `the ledger has unknown key "Narrative_Claims"`, 0},
		{"every_schema_key", writeLedger(`{"version":1,"description":"d","narrative_claims":[` +
			`{"id":"C1","source":"s","claim":"c","reality":"r","status":"reviewed-accurate","tracked_in":"",` +
			`"check":{"type":"present_artifact","path":"README.md","expect_present":true}}]}`), "", 1},
		{"required_keys_only", writeLedger(`{"version":1,"narrative_claims":[` + okClaim + `]}`), "", 1},
		{"no_claims", writeLedger(`{"version":1,"narrative_claims":[]}`), "", 0},
		{"present_artifact_default_expect", ledger(claimWith(base + `,"check":{"type":"present_artifact","path":"README.md"}`)), "", 1},
		{"tracked_in_empty", ledger(claimWith(base + `,"tracked_in":"","check":{"type":"manual"}`)), "", 1},
		{"description_empty", writeLedger(`{"version":1,"description":"","narrative_claims":[]}`), "", 0},
		{"version_missing", writeLedger(`{"narrative_claims":[]}`), parseErr + "version is missing", 0},
		{"version_wrong", writeLedger(`{"version":7,"narrative_claims":[]}`), parseErr + "version must be 1, got 7", 0},
		{"version_float", writeLedger(`{"version":1.0,"narrative_claims":[]}`), parseErr + "version must be 1, got 1.0", 0},
		{"version_string", writeLedger(`{"version":"1","narrative_claims":[]}`), parseErr + `version must be 1, got "1"`, 0},
		{"version_null", writeLedger(`{"version":null,"narrative_claims":[]}`), parseErr + "version is null", 0},
		{"version_exponent", writeLedger(`{"version":1e0,"narrative_claims":[]}`), parseErr + "version must be 1, got 1e0", 0},
		{"version_negative", writeLedger(`{"version":-1,"narrative_claims":[]}`), parseErr + "version must be 1, got -1", 0},
		{"version_leading_zero", writeLedger(`{"version":01,"narrative_claims":[]}`), parseErr + "the ledger is not valid JSON at byte 12", 0},
		{"description_not_string", writeLedger(`{"version":1,"description":1,"narrative_claims":[]}`), parseErr + "description must be a string", 0},
		{"description_null", writeLedger(`{"version":1,"description":null,"narrative_claims":[]}`), parseErr + "description is null", 0},
		{"claim_empty_object", ledger(`{}`), parseErr + "narrative_claims[0].check is missing", 0},
		{"claim_no_id", ledger(`{"source":"s","claim":"c","reality":"r","status":"reviewed-accurate","check":{"type":"manual"}}`), parseErr + "narrative_claims[0].id is missing", 0},
		{"claim_no_source", ledger(`{"id":"C1","claim":"c","reality":"r","status":"reviewed-accurate","check":{"type":"manual"}}`), parseErr + "narrative_claims[0].source is missing", 0},
		{"claim_no_claim", ledger(`{"id":"C1","source":"s","reality":"r","status":"reviewed-accurate","check":{"type":"manual"}}`), parseErr + "narrative_claims[0].claim is missing", 0},
		{"claim_no_reality", ledger(`{"id":"C1","source":"s","claim":"c","status":"reviewed-accurate","check":{"type":"manual"}}`), parseErr + "narrative_claims[0].reality is missing", 0},
		{"claim_no_status", ledger(`{"id":"C1","source":"s","claim":"c","reality":"r","check":{"type":"manual"}}`), parseErr + "narrative_claims[0].status is missing", 0},
		{"claim_no_check", ledger(claimWith(base)), parseErr + "narrative_claims[0].check is missing", 0},
		{"second_claim_no_id", ledger(okClaim, `{"source":"s","claim":"c","reality":"r","status":"reviewed-accurate","check":{"type":"manual"}}`), parseErr + "narrative_claims[1].id is missing", 0},
		{"id_blank", ledger(`{"id":" \t","source":"s","claim":"c","reality":"r","status":"reviewed-accurate","check":{"type":"manual"}}`), parseErr + "narrative_claims[0].id must be a non-blank string", 0},
		{"source_empty", ledger(`{"id":"C1","source":"","claim":"c","reality":"r","status":"reviewed-accurate","check":{"type":"manual"}}`), parseErr + "narrative_claims[0].source must be a non-blank string", 0},
		{"claim_text_blank", ledger(`{"id":"C1","source":"s","claim":" ","reality":"r","status":"reviewed-accurate","check":{"type":"manual"}}`), parseErr + "narrative_claims[0].claim must be a non-blank string", 0},
		{"reality_blank", ledger(`{"id":"C1","source":"s","claim":"c","reality":"","status":"reviewed-accurate","check":{"type":"manual"}}`), parseErr + "narrative_claims[0].reality must be a non-blank string", 0},
		{"id_null", ledger(`{"id":null,"source":"s","claim":"c","reality":"r","status":"reviewed-accurate","check":{"type":"manual"}}`), parseErr + "narrative_claims[0].id is null", 0},
		{"status_null", ledger(`{"id":"C1","source":"s","claim":"c","reality":"r","status":null,"check":{"type":"manual"}}`), parseErr + "narrative_claims[0].status is null", 0},
		{"tracked_in_null", ledger(claimWith(base + `,"tracked_in":null,"check":{"type":"manual"}`)), parseErr + "narrative_claims[0].tracked_in is null", 0},
		{"check_type_missing", ledger(claimWith(base + `,"check":{}`)), parseErr + "narrative_claims[0].check.type is missing", 0},
		{"check_type_null", ledger(claimWith(base + `,"check":{"type":null}`)), parseErr + "narrative_claims[0].check.type is null", 0},
		{"check_type_number", ledger(claimWith(base + `,"check":{"type":5}`)), parseErr + "narrative_claims[0].check.type must be a string", 0},
		{"check_type_array", ledger(claimWith(base + `,"check":{"type":[]}`)), parseErr + "narrative_claims[0].check.type must be a string", 0},
		{"check_type_object", ledger(claimWith(base + `,"check":{"type":{}}`)), parseErr + "narrative_claims[0].check.type must be a string", 0},
		{"check_type_bool", ledger(claimWith(base + `,"check":{"type":true}`)), parseErr + "narrative_claims[0].check.type must be a string", 0},
		{"check_type_empty", ledger(claimWith(base + `,"check":{"type":""}`)), parseErr + `narrative_claims[0].check.type is "", want one of manual|present_artifact`, 0},
		{"id_zero_width_escape", ledger(`{"id":"\u200b","source":"s","claim":"c","reality":"r","status":"reviewed-accurate","check":{"type":"manual"}}`), parseErr + "narrative_claims[0].id must be a non-blank string", 0},
		{"source_zero_width_raw", ledger(`{"id":"C1","source":"` + "\u200b\u200c" + `","claim":"c","reality":"r","status":"reviewed-accurate","check":{"type":"manual"}}`), parseErr + "narrative_claims[0].source must be a non-blank string", 0},
		{"claim_word_joiner_space", ledger(`{"id":"C1","source":"s","claim":" \u2060 ","reality":"r","status":"reviewed-accurate","check":{"type":"manual"}}`), parseErr + "narrative_claims[0].claim must be a non-blank string", 0},
		{"reality_byte_order_mark", ledger(`{"id":"C1","source":"s","claim":"c","reality":"\ufeff","status":"reviewed-accurate","check":{"type":"manual"}}`), parseErr + "narrative_claims[0].reality must be a non-blank string", 0},
		{"id_zero_width_inside", ledger(`{"id":"C\u200b1","source":"s","claim":"c","reality":"r","status":"reviewed-accurate","check":{"type":"manual"}}`), "", 1},
		{"id_lone_surrogate", ledger(`{"id":"\udc00","source":"s","claim":"c","reality":"r","status":"reviewed-accurate","check":{"type":"manual"}}`), parseErr + "narrative_claims[0].id must not contain U+FFFD or an unpaired surrogate", 0},
		{"source_replacement_char", ledger(`{"id":"C1","source":"s` + "\ufffd" + `","claim":"c","reality":"r","status":"reviewed-accurate","check":{"type":"manual"}}`), parseErr + "narrative_claims[0].source must not contain U+FFFD or an unpaired surrogate", 0},
		{"tracked_in_lone_surrogate", ledger(claimWith(base + `,"tracked_in":"#1\ud800","check":{"type":"manual"}`)), parseErr + "narrative_claims[0].tracked_in must not contain U+FFFD or an unpaired surrogate", 0},
		{"status_replacement_char", ledger(`{"id":"C1","source":"s","claim":"c","reality":"r","status":"reviewed-accurate\ufffd","check":{"type":"manual"}}`), parseErr + "narrative_claims[0].status must not contain U+FFFD or an unpaired surrogate", 0},
		{"check_type_typo", ledger(claimWith(base + `,"check":{"type":"present_artfact","path":"nope.md"}`)), parseErr + `narrative_claims[0].check.type is "present_artfact", want one of manual|present_artifact`, 0},
		{"check_type_case", ledger(claimWith(base + `,"check":{"type":"Manual"}`)), parseErr + `narrative_claims[0].check.type is "Manual", want one of manual|present_artifact`, 0},
		{"manual_with_path", ledger(claimWith(base + `,"check":{"type":"manual","path":"README.md"}`)), parseErr + `narrative_claims[0].check has unknown key "path" for type "manual"`, 0},
		{"manual_with_expect", ledger(claimWith(base + `,"check":{"type":"manual","expect_present":false}`)), parseErr + `narrative_claims[0].check has unknown key "expect_present" for type "manual"`, 0},
		{"present_artifact_no_path", ledger(claimWith(base + `,"check":{"type":"present_artifact","expect_present":false}`)), parseErr + "narrative_claims[0].check.path is missing", 0},
		{"present_artifact_blank_path", ledger(claimWith(base + `,"check":{"type":"present_artifact","path":" "}`)), parseErr + "narrative_claims[0].check.path must be a non-blank string", 0},
		{"present_artifact_null_path", ledger(claimWith(base + `,"check":{"type":"present_artifact","path":null}`)), parseErr + "narrative_claims[0].check.path is null", 0},
		{"expect_present_null", ledger(claimWith(base + `,"check":{"type":"present_artifact","path":"README.md","expect_present":null}`)), parseErr + "narrative_claims[0].check.expect_present is null", 0},
		{"id_not_string", ledger(`{"id":5,"source":"s","claim":"c","reality":"r","status":"reviewed-accurate","check":{"type":"manual"}}`), parseErr + "narrative_claims[0].id must be a non-blank string", 0},
		{"status_not_string", ledger(`{"id":"C1","source":"s","claim":"c","reality":"r","status":1,"check":{"type":"manual"}}`), parseErr + "narrative_claims[0].status must be a string", 0},
		{"tracked_in_not_string", ledger(claimWith(base + `,"tracked_in":17,"check":{"type":"manual"}`)), parseErr + "narrative_claims[0].tracked_in must be a string", 0},
		{"expect_present_not_bool", ledger(claimWith(base + `,"check":{"type":"present_artifact","path":"README.md","expect_present":"false"}`)), parseErr + "narrative_claims[0].check.expect_present must be true or false", 0},
		{"trailing_data", writeLedger(`{"version":1,"narrative_claims":[]} {}`), parseErr + "the ledger has data after its top-level value at byte 36", 0},
		{"dup_top_scalar", writeLedger(`{"version":2,"version":1,"narrative_claims":[]}`), parseErr + `the ledger has duplicate key "version"`, 0},
		{"dup_claims_array", writeLedger(`{"version":1,"narrative_claims":[` + okClaim + `,` + okClaim + `],"narrative_claims":[{"id":"C2"}]}`), parseErr + `the ledger has duplicate key "narrative_claims"`, 0},
		{"dup_claim_key", ledger(claimWith(base + `,"status":"unreviewed","check":{"type":"manual"}`)), parseErr + `narrative_claims[0] has duplicate key "status"`, 0},
		{"dup_check_object", ledger(claimWith(base + `,"check":{"bogus":1},"check":{"type":"manual"}`)), parseErr + `narrative_claims[0] has duplicate key "check"`, 0},
		{"dup_check_key", ledger(claimWith(base + `,"check":{"type":"present_artifact","path":"README.md","path":"nope.md"}`)), parseErr + `narrative_claims[0].check has duplicate key "path"`, 0},
		{"dup_key_escaped", ledger(`{"id":"C1","\u0069d":"C2","source":"s","claim":"c","reality":"r","status":"reviewed-accurate","check":{"type":"manual"}}`), parseErr + `narrative_claims[0] has duplicate key "id"`, 0},
		{"dup_second_claim", ledger(okClaim, claimWith(base+`,"id":"C2","check":{"type":"manual"}`)), parseErr + `narrative_claims[1] has duplicate key "id"`, 0},
	}
	wrapsNotExist := map[string]bool{
		"dangling_symlink":         true,
		"dangling_symlink_chain":   true,
		"dangling_conformance_dir": true,
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			files := baseTree()
			delete(files, "conformance/claims.json")
			root := writeTree(t, files)
			if err := os.MkdirAll(filepath.Join(root, "conformance"), 0o755); err != nil {
				t.Fatal(err)
			}
			tc.setup(t, filepath.Join(root, "conformance", "claims.json"))

			r := analyze(root)
			out := report(r)
			if tc.wantPrefix == "" {
				if len(r.errors) != 0 || !strings.Contains(out, "RESULT: PASS") {
					t.Errorf("want a pass with no errors, got: %v", r.errors)
				}
				if len(r.ledger) != tc.wantClaims {
					t.Errorf("ledger claims = %d, want %d", len(r.ledger), tc.wantClaims)
				}
				return
			}
			if len(r.errors) != 1 || !strings.HasPrefix(r.errors[0], tc.wantPrefix) {
				t.Errorf("want one error starting %q, got: %v", tc.wantPrefix, r.errors)
			}
			if strings.HasPrefix(tc.wantPrefix, parseErr) && len(r.errors) == 1 && r.errors[0] != tc.wantPrefix {
				t.Errorf("a parse error is the checker's own text, want exactly %q, got %q", tc.wantPrefix, r.errors[0])
			}
			if len(r.ledger) != 0 {
				t.Errorf("a ledger that failed to load must be empty, got %d claims", len(r.ledger))
			}
			if !strings.Contains(out, "RESULT: FAIL") {
				t.Errorf("want RESULT: FAIL, got:\n%s", out)
			}
			if wrapsNotExist[tc.name] {
				if _, err := loadLedger(root); !errors.Is(err, fs.ErrNotExist) {
					t.Errorf("want an error that wraps fs.ErrNotExist, got: %v", err)
				}
			}
		})
	}
}

// TestLedgerLoadRoot covers plugin roots that are not plugin trees: a root
// that does not resolve (LL-10), a directory with no Crosscheck manifest
// (LL-11), and a root the ledger read cannot open (LL-2).
func TestLedgerLoadRoot(t *testing.T) {
	symlink := func(t *testing.T, target, path string) {
		if err := os.Symlink(target, path); err != nil {
			t.Fatal(err)
		}
	}
	withManifest := func(manifest string) func(*testing.T, string) string {
		return func(t *testing.T, _ string) string {
			files := baseTree()
			files[".claude-plugin/plugin.json"] = manifest
			return writeTree(t, files)
		}
	}
	tests := []struct {
		name     string
		root     func(t *testing.T, base string) string
		wantErrs []string
	}{
		{"missing_root", func(_ *testing.T, base string) string {
			return filepath.Join(base, "missing")
		}, []string{"LL-10"}},
		{"dangling_root", func(t *testing.T, base string) string {
			root := filepath.Join(base, "root")
			symlink(t, "missing", root)
			return root
		}, []string{"LL-10"}},
		{"dangling_ancestor", func(t *testing.T, base string) string {
			symlink(t, "missing", filepath.Join(base, "repo"))
			return filepath.Join(base, "repo", "crosscheck")
		}, []string{"LL-10"}},
		{"empty_root", func(_ *testing.T, base string) string {
			return base
		}, []string{"LL-11"}},
		{"no_manifest", func(t *testing.T, _ string) string {
			root := writeTree(t, map[string]string{"README.md": "Not a plugin."})
			if err := os.Mkdir(filepath.Join(root, "skills"), 0o755); err != nil {
				t.Fatal(err)
			}
			return root
		}, []string{"LL-11"}},
		{"manifest_not_json", withManifest("name: crosscheck"), []string{"LL-11"}},
		{"manifest_other_name", withManifest(`{"name":"cloudflare"}`), []string{"LL-11"}},
		{"root_symlink_to_tree", func(t *testing.T, base string) string {
			files := baseTree()
			delete(files, "conformance/claims.json")
			root := filepath.Join(base, "root")
			symlink(t, writeTree(t, files), root)
			return root
		}, nil},
		{"root_regular_file", func(t *testing.T, base string) string {
			root := filepath.Join(base, "root")
			if err := os.WriteFile(root, []byte("a file"), 0o644); err != nil {
				t.Fatal(err)
			}
			return root
		}, []string{"LL-2"}},
		{"root_symlink_loop", func(t *testing.T, base string) string {
			symlink(t, "b", filepath.Join(base, "a"))
			symlink(t, "a", filepath.Join(base, "b"))
			return filepath.Join(base, "a")
		}, []string{"LL-2"}},
		{"root_no_permission", func(t *testing.T, _ string) string {
			if os.Geteuid() == 0 {
				t.Skip("root can read a mode-000 directory")
			}
			root := writeTree(t, baseTree())
			if err := os.Chmod(root, 0); err != nil {
				t.Fatal(err)
			}
			t.Cleanup(func() { _ = os.Chmod(root, 0o755) })
			return root
		}, []string{"LL-11", "LL-2"}},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			root := tc.root(t, t.TempDir())
			r := analyze(root)
			out := report(r)
			if len(tc.wantErrs) == 0 {
				if len(r.errors) != 0 || !strings.Contains(out, "RESULT: PASS") {
					t.Errorf("want a pass with no errors, got: %v", r.errors)
				}
				return
			}
			prefix := map[string]string{
				"LL-2":  "[ledger] cannot read conformance/claims.json: ",
				"LL-10": "[ledger] cannot read conformance/claims.json: plugin root " + root + " does not resolve: ",
				"LL-11": "[root] plugin root " + root + " is not a Crosscheck plugin tree: ",
			}
			if len(r.errors) != len(tc.wantErrs) {
				t.Fatalf("want %d errors (%v), got: %v", len(tc.wantErrs), tc.wantErrs, r.errors)
			}
			for i, rule := range tc.wantErrs {
				if !strings.HasPrefix(r.errors[i], prefix[rule]) {
					t.Errorf("error %d: want %s, starting %q, got: %q", i, rule, prefix[rule], r.errors[i])
				}
				if rule != "LL-10" && strings.Contains(r.errors[i], "does not resolve") {
					t.Errorf("error %d: want %s, got an LL-10 error: %q", i, rule, r.errors[i])
				}
			}
			if !strings.Contains(out, "RESULT: FAIL") {
				t.Errorf("want RESULT: FAIL, got:\n%s", out)
			}
			if tc.wantErrs[0] == "LL-10" {
				if _, err := loadLedger(root); !errors.Is(err, fs.ErrNotExist) {
					t.Errorf("want an error that wraps fs.ErrNotExist, got: %v", err)
				}
			}
		})
	}
}

func TestReportPassFail(t *testing.T) {
	pass := report(result{})
	if !strings.Contains(pass, "RESULT: PASS") {
		t.Errorf("empty result should report PASS, got:\n%s", pass)
	}
	fail := report(result{errors: []string{"boom"}})
	if !strings.Contains(fail, "RESULT: FAIL") {
		t.Errorf("result with errors should report FAIL, got:\n%s", fail)
	}
}

// TestGoldenRealTree pins the oracle's verdict against the actual crosscheck/
// plugin tree (the parent of this package dir). It asserts the stable inventory
// and the post-fix gate state: assurance-probe now has frontmatter, the
// journal-context orphan WARNING remains a human decision, and all seven ledger
// claims hold (three of them known-gap present_artifact/manual checks;
// CLAIM-METHODOLOGY-COMMITTED and CLAIM-SELF-COVERAGE both triaged to
// reviewed-disclosed per epic #217 / issue #221).
func TestGoldenRealTree(t *testing.T) {
	root := ".." // package dir is crosscheck/conformance; plugin root is crosscheck/
	if _, err := os.Stat(filepath.Join(root, "skills")); err != nil {
		t.Skipf("real plugin tree not found at %s: %v", root, err)
	}
	r := analyze(root)

	if len(r.skills) != 30 {
		t.Errorf("skills discovered = %d, want 30", len(r.skills))
	}
	if len(r.agents) != 5 {
		t.Errorf("agents discovered = %d, want 5 (byfuglien, hellebuyck, add-orchestrator, lowry, auditor)", len(r.agents))
	}
	if len(r.refTokens) != 30 {
		t.Errorf("referenced tokens = %d, want 30", len(r.refTokens))
	}
	if len(r.ledger) != 7 {
		t.Fatalf("ledger claims = %d, want 7", len(r.ledger))
	}

	// journal-context is now documented in the README skills overview, so it must
	// no longer be flagged as an orphan.
	if hasMatch(r.warnings, "journal-context") {
		t.Errorf("journal-context should be documented, but is still flagged as an orphan: %v", r.warnings)
	}

	// All five originally-known-gap claims were triaged to reviewed-disclosed as
	// their backing artifacts shipped: CLAIM-PHASE4 (agents/lowry.md, #218),
	// CLAIM-AUDITOR (agents/auditor.md, #220), CLAIM-MODES (operating modes, #219),
	// CLAIM-METHODOLOGY-COMMITTED (archived per epic #217), and CLAIM-SELF-COVERAGE
	// (AUTO 5 orchestration-graph integrity, #221). No known-gap claims remain.
	for _, id := range []string{"CLAIM-PHASE4", "CLAIM-AUDITOR", "CLAIM-MODES", "CLAIM-METHODOLOGY-COMMITTED", "CLAIM-SELF-COVERAGE"} {
		found := false
		for _, c := range r.ledger {
			if c.ID == id {
				found = true
				if c.Status != "reviewed-disclosed" {
					t.Errorf("claim %s status = %q, want reviewed-disclosed", id, c.Status)
				}
			}
		}
		if !found {
			t.Errorf("missing expected ledger claim %s", id)
		}
	}

	// Post-fix expectation: the gate is GREEN. assurance-probe now parses, and the
	// present_artifact ledger checks (lowry/methodology/auditor absent) all hold,
	// so there are no ERRORs.
	if hasMatch(r.errors, "assurance-probe") {
		t.Errorf("assurance-probe should have valid frontmatter post-fix; errors: %v", r.errors)
	}
	if len(r.errors) != 0 {
		t.Errorf("expected RESULT PASS (0 errors) post-fix, got %d: %v", len(r.errors), r.errors)
	}
}
