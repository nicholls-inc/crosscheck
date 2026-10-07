// Command conformance is the Crosscheck conformance / inventory oracle.
//
// It verifies that what the documentation CLAIMS Crosscheck ships matches what
// the filesystem actually contains. This is the bidirectional-coverage-gate
// pattern (docs/invariants <-> tests) lifted to the meta level: docs <->
// artifacts.
//
// Two layers of check:
//
//	AUTO   - deterministic, no false positives, fail CI on ERROR
//	         (artifact<->doc reference integrity + structural integrity)
//	LEDGER - narrative claims from claims.json that can't be auto-verified
//	         (layer/phase/mode counts, terminal states). Surfaced for review,
//	         dated, and tracked. Drift here is a known-gap, not a silent one.
//
// Exit 1 if any AUTO check is ERROR. LEDGER never fails CI by itself, but any
// ledger entry with status 'unreviewed' is promoted to an ERROR (forces
// triage), and a 'present_artifact' auto-check that disagrees with reality is
// likewise promoted to an ERROR.
//
// A ledger claim's status must be one of these four, matched exactly:
//
//	unreviewed         - new and not yet triaged; an ERROR until triaged
//	known-gap          - the claim does not hold yet; needs a tracked_in link
//	reviewed-disclosed - reviewed, and the docs disclose where reality differs
//	reviewed-accurate  - reviewed, and the claim holds as written
//
// Any other status, including an empty one, is an ERROR, so a typo such as
// 'reviewed-disclsed' cannot pass as a reviewed claim.
//
// A missing claims.json is an empty ledger. A claims.json that cannot be read,
// including a symlink to a missing target, or does not parse as the ledger
// schema, is an ERROR. The schema (ledgerSchema, claimSchema, checkSchemas):
//
//	top level - version (the number 1) and narrative_claims (an array) are
//	            required; description (a string) is optional
//	claim     - id, source, claim and reality (non-blank strings), status (a
//	            string) and check (an object) are required; tracked_in (a
//	            string) is optional
//	check     - type is manual or present_artifact; manual takes no other key;
//	            present_artifact requires path (a non-blank string) and takes
//	            an optional expect_present (true or false, default true)
//
// Keys match exactly, including case. A key the schema does not name, a key
// that appears twice in one object, and a null anywhere are ERRORs. A
// claims.json or a conformance directory that is a symlink to a missing target
// is an ERROR, and so is a plugin root that does not resolve, because it is
// missing or a symlink on its path dangles. A plugin root is a directory whose
// .claude-plugin/plugin.json names crosscheck, and any other directory is an
// ERROR.
//
// Not yet reached: what the text fields say. Two claims may share an id,
// source and tracked_in need not name a real file or issue, and check.path may
// point outside the plugin root. The property that blocks it is a check of
// each field against the tree and the tracker; the open question is which of
// them can be checked without a network call. Unique ids are PB-1.42.
package main

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
)

// docFiles is the user-facing documentation set scanned for references. Files
// that do not exist are skipped.
var docFiles = []string{
	"README.md",
	"docs/skills.md",
	"docs/agents.md",
	"docs/assurance-hierarchy.md",
	"docs/research/assurance-hierarchy.md",
}

// knownStatus is the ledger status allowlist the header comment documents.
var knownStatus = map[string]bool{
	"unreviewed":         true,
	"known-gap":          true,
	"reviewed-disclosed": true,
	"reviewed-accurate":  true,
}

// reqKeys are the frontmatter keys every skill and agent must declare.
var reqKeys = []string{"name", "description"}

var (
	fmLineRe    = regexp.MustCompile(`^([a-zA-Z0-9_-]+):\s*(.*)$`)
	slashTokRe  = regexp.MustCompile("\x60/([a-z][a-z0-9-]+)\x60")
	xcheckRe    = regexp.MustCompile(`/crosscheck:([a-z][a-z0-9-]+)`)
	dafnyToolRe = regexp.MustCompile("\x60(dafny_[a-z]+)\x60")
)

// skill is a discovered skills/<dir>/SKILL.md artifact.
type skill struct {
	name  string
	fm    map[string]string
	empty bool
}

// agent is a discovered agents/<stem>.md artifact. body is the agent content
// with the leading YAML frontmatter stripped, so routing checks scan prose only
// and never the description (where a `/skill` token is documentation, not a
// routing edge).
type agent struct {
	name string
	fm   map[string]string
	body string
}

// claim is one narrative-ledger entry from claims.json.
type claim struct {
	ID        string     `json:"id"`
	Source    string     `json:"source"`
	Claim     string     `json:"claim"`
	Reality   string     `json:"reality"`
	Status    string     `json:"status"`
	Check     claimCheck `json:"check"`
	TrackedIn string     `json:"tracked_in"`
}

type claimCheck struct {
	Type          string `json:"type"`
	Path          string `json:"path"`
	ExpectPresent *bool  `json:"expect_present"`
}

type ledgerFile struct {
	NarrativeClaims []claim `json:"narrative_claims"`
}

// result holds everything the oracle discovered and decided, so that report
// rendering and the process exit code are pure functions of it.
type result struct {
	skills      []skill
	agents      []agent
	presentDocs []string
	refTokens   []string
	errors      []string
	warnings    []string
	ledger      []claim
}

// parseFrontmatter parses a leading YAML frontmatter block (the text between
// the opening "---" and the next "\n---") into a key->value map. It tolerates
// folded scalars such as `description: >-` (the value is captured verbatim as
// ">-", which is non-empty, so the key counts as present). It returns the
// parsed map and the original content unchanged.
func parseFrontmatter(content string) (map[string]string, string) {
	if !strings.HasPrefix(content, "---") {
		return map[string]string{}, content
	}
	rel := strings.Index(content[3:], "\n---")
	if rel == -1 {
		return map[string]string{}, content
	}
	end := 3 + rel
	fm := map[string]string{}
	for _, line := range strings.Split(content[3:end], "\n") {
		if m := fmLineRe.FindStringSubmatch(line); m != nil {
			fm[m[1]] = strings.TrimSpace(m[2])
		}
	}
	return fm, content
}

// stripFrontmatter returns content with a leading YAML frontmatter block (the
// text from the opening "---" through the next "\n---" line) removed. If there
// is no well-formed frontmatter block, the content is returned unchanged. This
// is the body that routing checks scan, so a `/skill` token in a frontmatter
// description is never mistaken for a routing edge.
func stripFrontmatter(content string) string {
	if !strings.HasPrefix(content, "---") {
		return content
	}
	rel := strings.Index(content[3:], "\n---")
	if rel == -1 {
		return content
	}
	// Skip past the closing "---" line to the start of the body.
	rest := content[3+rel+len("\n---"):]
	if nl := strings.IndexByte(rest, '\n'); nl != -1 {
		return rest[nl+1:]
	}
	return ""
}

// readFile reads a file as a string, mirroring Python's errors="replace": any
// read error yields the empty string rather than aborting.
func readFile(path string) string {
	b, err := os.ReadFile(path)
	if err != nil {
		return ""
	}
	return string(b)
}

func fileExists(path string) bool {
	_, err := os.Stat(path)
	return err == nil
}

// discoverSkills returns every subdir of skills/ that holds a SKILL.md, sorted
// by directory name.
func discoverSkills(root string) []skill {
	var out []skill
	entries, err := os.ReadDir(filepath.Join(root, "skills"))
	if err != nil {
		return out
	}
	var names []string
	for _, e := range entries {
		if e.IsDir() {
			names = append(names, e.Name())
		}
	}
	sort.Strings(names)
	for _, name := range names {
		sk := filepath.Join(root, "skills", name, "SKILL.md")
		if !fileExists(sk) {
			continue
		}
		fm, body := parseFrontmatter(readFile(sk))
		out = append(out, skill{
			name:  name,
			fm:    fm,
			empty: len(strings.TrimSpace(body)) < 50,
		})
	}
	return out
}

// discoverAgents returns every agents/*.md artifact, sorted by file stem.
func discoverAgents(root string) []agent {
	var out []agent
	matches, err := filepath.Glob(filepath.Join(root, "agents", "*.md"))
	if err != nil {
		return out
	}
	sort.Strings(matches)
	for _, f := range matches {
		content := readFile(f)
		fm, _ := parseFrontmatter(content)
		stem := strings.TrimSuffix(filepath.Base(f), ".md")
		out = append(out, agent{name: stem, fm: fm, body: stripFrontmatter(content)})
	}
	return out
}

// scanDocs concatenates the existing doc-set files and records which were
// present, mirroring the reference (a leading "\n" before each file's text).
func scanDocs(root string) (docText string, present []string) {
	for _, rel := range docFiles {
		p := filepath.Join(root, rel)
		if fileExists(p) {
			docText += "\n" + readFile(p)
			present = append(present, rel)
		}
	}
	return docText, present
}

// referencedTokens extracts the set of `/token` and /crosscheck:token tokens
// referenced anywhere in the doc text, sorted.
func referencedTokens(docText string) []string {
	set := map[string]struct{}{}
	for _, m := range slashTokRe.FindAllStringSubmatch(docText, -1) {
		set[m[1]] = struct{}{}
	}
	for _, m := range xcheckRe.FindAllStringSubmatch(docText, -1) {
		set[m[1]] = struct{}{}
	}
	out := make([]string, 0, len(set))
	for t := range set {
		out = append(out, t)
	}
	sort.Strings(out)
	return out
}

// documented reports whether an artifact name appears in the doc text in any
// of the forms the reference recognises.
func documented(name, docText string) bool {
	return strings.Contains(docText, "`/"+name+"`") ||
		strings.Contains(docText, "crosscheck:"+name) ||
		strings.Contains(docText, "`"+name+"`") ||
		strings.Contains(docText, "/"+name+" ")
}

// pyList renders a string slice the way Python repr does: ['a', 'b'].
func pyList(items []string) string {
	parts := make([]string, len(items))
	for i, s := range items {
		parts[i] = "'" + s + "'"
	}
	return "[" + strings.Join(parts, ", ") + "]"
}

// missingKeys returns the required keys absent from fm, sorted.
func missingKeys(fm map[string]string) []string {
	var missing []string
	for _, k := range reqKeys {
		if _, ok := fm[k]; !ok {
			missing = append(missing, k)
		}
	}
	sort.Strings(missing)
	return missing
}

// analyze runs every AUTO and LEDGER check against the plugin tree rooted at
// root and returns the assembled result.
func analyze(root string) result {
	var r result
	if err := checkPluginRoot(root); err != nil {
		r.errors = append(r.errors, "[root] "+err.Error())
	}
	r.skills = discoverSkills(root)
	r.agents = discoverAgents(root)
	docText, present := scanDocs(root)
	r.presentDocs = present
	r.refTokens = referencedTokens(docText)

	known := map[string]struct{}{}
	for _, s := range r.skills {
		known[s.name] = struct{}{}
	}
	for _, a := range r.agents {
		known[a.name] = struct{}{}
	}

	// ---- AUTO 1: structural integrity ----
	for _, s := range r.skills {
		if missing := missingKeys(s.fm); len(missing) > 0 {
			r.errors = append(r.errors, fmt.Sprintf(
				"[structural] skill '%s': SKILL.md missing frontmatter keys %s", s.name, pyList(missing)))
		}
		if n := s.fm["name"]; n != "" && n != s.name {
			r.errors = append(r.errors, fmt.Sprintf(
				"[structural] skill dir '%s' != frontmatter name '%s'", s.name, n))
		}
		if s.empty {
			r.errors = append(r.errors, fmt.Sprintf(
				"[structural] skill '%s': SKILL.md is effectively empty", s.name))
		}
	}
	for _, a := range r.agents {
		if missing := missingKeys(a.fm); len(missing) > 0 {
			r.errors = append(r.errors, fmt.Sprintf(
				"[structural] agent '%s': missing frontmatter keys %s", a.name, pyList(missing)))
		}
		if n := a.fm["name"]; n != "" && n != a.name {
			r.errors = append(r.errors, fmt.Sprintf(
				"[structural] agent file '%s.md' != frontmatter name '%s'", a.name, n))
		}
	}

	// ---- AUTO 2: phantom (doc references an artifact that does not exist) ----
	for _, tok := range r.refTokens {
		if _, ok := known[tok]; !ok {
			r.errors = append(r.errors, fmt.Sprintf(
				"[phantom] docs reference '/%s' but no skills/%s/ or agents/%s.md exists", tok, tok, tok))
		}
	}

	// ---- AUTO 3: orphan (artifact exists but is referenced nowhere) ----
	for _, s := range r.skills {
		if !documented(s.name, docText) {
			r.warnings = append(r.warnings, fmt.Sprintf(
				"[orphan] skill '%s' ships but is not referenced in any doc %s", s.name, pyList(present)))
		}
	}
	for _, a := range r.agents {
		if !documented(a.name, docText) {
			r.warnings = append(r.warnings, fmt.Sprintf(
				"[orphan] agent '%s' ships but is not referenced in any doc", a.name))
		}
	}

	// ---- AUTO 4: MCP tools claimed vs mcp-server source ----
	mcpSrc := readMCPSource(root)
	for _, m := range dafnyToolRe.FindAllStringSubmatch(docText, -1) {
		tool := m[1]
		if mcpSrc != "" && !strings.Contains(mcpSrc, tool) {
			r.warnings = append(r.warnings, fmt.Sprintf(
				"[mcp] README claims MCP tool '%s' but it's not found in mcp-server source", tool))
		}
	}

	// ---- AUTO 5: orchestration-graph integrity (trunk self-coverage) ----
	// The phantom check (AUTO 2) only scans the user-facing doc set, so an
	// orchestrator that routes to a skill/agent which does not exist would slip
	// through. This extends reference integrity to the *trunk*: every skill or
	// agent that an agent's body routes to (via `/crosscheck:x` or `/x`) must
	// resolve to a real artifact. It is the second trunk-level self-check after
	// this oracle itself (see CLAIM-SELF-COVERAGE, issue #221).
	//
	// Only the agent *body* is scanned (a.body has frontmatter stripped): a
	// `/skill` token in a frontmatter description is documentation, not a routing
	// edge, and must not raise a routing error.
	//
	// Resolution is against the `known` set — skills/agents that registered as
	// real artifacts (a skills/<x>/SKILL.md or agents/<x>.md with valid
	// frontmatter). The error wording reflects that: a directory that exists but
	// lacks a parseable SKILL.md does not register and is reported as unresolved.
	for _, a := range r.agents {
		for _, tok := range referencedTokens(a.body) {
			if _, ok := known[tok]; !ok {
				r.errors = append(r.errors, fmt.Sprintf(
					"[routing] agent '%s' routes to '/%s' but no registered skill/agent '%s' resolves "+
						"(needs skills/%s/SKILL.md or agents/%s.md with valid frontmatter)",
					a.name, tok, tok, tok, tok))
			}
		}
	}

	// ---- AUTO 6: operating-mode tag coverage (ADD modes, #219) ----
	// (AUTO 5 — orchestration-graph integrity — is owned by sibling PR #230.)
	// Every load-bearing module (skill + agent) must declare a valid `add-mode`
	// frontmatter tag in {add, bootstrap}. This lifts the A3 acceptance oracle's
	// pass condition (conformance/acceptance) into the blocking lane: the mode
	// system is only "wired" if every module actually carries its tag, and a new
	// untagged module fails CI rather than silently opting out. ADR-001 governs
	// the taxonomy; crosscheck dogfoods it on its own skills/agents (existing
	// artifacts are `bootstrap` — governance retrofitted; ADD-built agents are
	// `add`). `transitional` describes a repo, never a module (operating-modes.md
	// + ADR-001), so it is deliberately NOT a valid per-module tag.
	validMode := map[string]bool{"add": true, "bootstrap": true}
	for _, s := range r.skills {
		if !validMode[s.fm["add-mode"]] {
			r.errors = append(r.errors, fmt.Sprintf(
				"[mode] skill '%s' has no valid add-mode tag (got %q, want one of add|bootstrap)", s.name, s.fm["add-mode"]))
		}
	}
	for _, a := range r.agents {
		if !validMode[a.fm["add-mode"]] {
			r.errors = append(r.errors, fmt.Sprintf(
				"[mode] agent '%s' has no valid add-mode tag (got %q, want one of add|bootstrap)", a.name, a.fm["add-mode"]))
		}
	}

	// ---- LEDGER: narrative claims ----
	ledger, err := loadLedger(root)
	if err != nil {
		r.errors = append(r.errors, "[ledger] "+err.Error())
	}
	r.ledger = ledger
	for _, c := range r.ledger {
		if !knownStatus[c.Status] {
			r.errors = append(r.errors, fmt.Sprintf(
				"[ledger] claim %s has unknown status %q (want one of unreviewed|known-gap|reviewed-disclosed|reviewed-accurate)",
				c.ID, c.Status))
		}
		if c.Status == "unreviewed" {
			r.errors = append(r.errors, fmt.Sprintf(
				"[ledger] claim %s is UNREVIEWED — triage required", c.ID))
		}
		if c.Status == "known-gap" && strings.TrimSpace(c.TrackedIn) == "" {
			r.errors = append(r.errors, fmt.Sprintf(
				"[ledger] claim %s is a known-gap with no tracked_in link", c.ID))
		}
		if c.Check.Type == "present_artifact" {
			expect := true
			if c.Check.ExpectPresent != nil {
				expect = *c.Check.ExpectPresent
			}
			exists := fileExists(filepath.Join(root, c.Check.Path))
			if exists != expect {
				r.errors = append(r.errors, fmt.Sprintf(
					"[ledger] claim %s auto-check failed: %s present=%v, expected_present=%v",
					c.ID, c.Check.Path, exists, expect))
			}
		}
	}

	return r
}

// checkPluginRoot reports a root directory whose .claude-plugin/plugin.json
// cannot be read, does not decode, or does not name crosscheck. A root that is
// not a directory to os.Stat passes here, because the ledger read reports it.
func checkPluginRoot(root string) error {
	if info, err := os.Stat(root); err != nil || !info.IsDir() {
		return nil
	}
	data, err := os.ReadFile(filepath.Join(root, ".claude-plugin", "plugin.json"))
	if err != nil {
		return fmt.Errorf("plugin root %s is not a Crosscheck plugin tree: %w", root, err)
	}
	var manifest struct {
		Name string `json:"name"`
	}
	if err := json.Unmarshal(data, &manifest); err != nil {
		return fmt.Errorf("plugin root %s is not a Crosscheck plugin tree: .claude-plugin/plugin.json: %w", root, err)
	}
	if manifest.Name != "crosscheck" {
		return fmt.Errorf("plugin root %s is not a Crosscheck plugin tree: .claude-plugin/plugin.json names %q, want \"crosscheck\"", root, manifest.Name)
	}
	return nil
}

// readMCPSource concatenates every .ts and .js file under mcp-server/, or ""
// if the directory does not exist.
func readMCPSource(root string) string {
	dir := filepath.Join(root, "mcp-server")
	if !fileExists(dir) {
		return ""
	}
	var sb strings.Builder
	_ = filepath.WalkDir(dir, func(path string, d os.DirEntry, err error) error {
		if err != nil || d.IsDir() {
			return nil
		}
		if strings.HasSuffix(path, ".ts") || strings.HasSuffix(path, ".js") {
			sb.WriteString(readFile(path))
		}
		return nil
	})
	return sb.String()
}

// loadLedger reads conformance/claims.json. A missing file is an empty ledger;
// a file that cannot be read, cannot be parsed, or breaks the ledger schema
// (checkLedgerSchema) is an error, so a broken ledger fails
// the run instead of passing with zero claims. A symlink to a missing target,
// at claims.json or at conformance, cannot be read, and neither can a ledger
// under a root that does not resolve.
func loadLedger(root string) ([]claim, error) {
	dir := filepath.Join(root, "conformance")
	path := filepath.Join(dir, "claims.json")
	data, err := os.ReadFile(path)
	if errors.Is(err, fs.ErrNotExist) {
		for _, p := range []string{path, dir} {
			if danglingSymlink(p) {
				return nil, fmt.Errorf("cannot read conformance/claims.json: %s is a symbolic link to a missing target: %w", p, err)
			}
		}
		if _, serr := os.Stat(root); serr != nil {
			return nil, fmt.Errorf("cannot read conformance/claims.json: plugin root %s does not resolve: %w", root, serr)
		}
		return nil, nil
	}
	if err != nil {
		return nil, fmt.Errorf("cannot read conformance/claims.json: %w", err)
	}
	if err := checkLedgerSchema(data); err != nil {
		return nil, fmt.Errorf("cannot parse conformance/claims.json: %w", err)
	}
	var lf ledgerFile
	if err := json.Unmarshal(data, &lf); err != nil {
		return nil, fmt.Errorf("cannot parse conformance/claims.json: %w", err)
	}
	return lf.NarrativeClaims, nil
}

// field is one key of the ledger schema: whether an object must carry it, and
// the kind its value must have.
type field struct {
	required bool
	kind     kind
}

// schema maps each key an object may carry to its field. Key names match
// exactly, because json.Unmarshal matches them without regard to case and
// drops any key it cannot place.
type schema map[string]field

// kind returns "" for a value of the right kind, or the fault. A null never
// reaches a kind: checkFields rejects it first.
type kind func(raw json.RawMessage) string

func nonBlank(raw json.RawMessage) string {
	var s string
	if json.Unmarshal(raw, &s) != nil || strings.TrimSpace(s) == "" {
		return "must be a non-blank string"
	}
	return ""
}

func anyString(raw json.RawMessage) string {
	var s string
	if json.Unmarshal(raw, &s) != nil {
		return "must be a string"
	}
	return ""
}

func boolean(raw json.RawMessage) string {
	var b bool
	if json.Unmarshal(raw, &b) != nil {
		return "must be true or false"
	}
	return ""
}

// versionOne accepts only the number 1 written as 1, the one ledger version.
func versionOne(raw json.RawMessage) string {
	if string(raw) != "1" {
		return "must be 1, got " + string(raw)
	}
	return ""
}

// nested is the kind of an object or array that checkLedgerSchema walks itself.
func nested(json.RawMessage) string { return "" }

var (
	ledgerSchema = schema{
		"version":          {true, versionOne},
		"description":      {false, anyString},
		"narrative_claims": {true, nested},
	}
	claimSchema = schema{
		"id":         {true, nonBlank},
		"source":     {true, nonBlank},
		"claim":      {true, nonBlank},
		"reality":    {true, nonBlank},
		"status":     {true, anyString},
		"check":      {true, nested},
		"tracked_in": {false, anyString},
	}
	// checkSchemas holds one schema per check.type. A type not listed here is
	// an error, so a misspelt type cannot pass as a check that never runs.
	checkSchemas = map[string]schema{
		"manual": {
			"type": {true, nonBlank},
		},
		"present_artifact": {
			"type":           {true, nonBlank},
			"path":           {true, nonBlank},
			"expect_present": {false, boolean},
		},
	}
)

// checkLedgerSchema rejects JSON that is not a ledger: an object that repeats a
// key, a key the schema does not name, a required key that is absent, a null,
// or a value of the wrong kind. It runs before json.Unmarshal, which would
// drop a repeated key, fold case and read null as a zero value.
func checkLedgerSchema(data []byte) error {
	top, err := readObject(data, "the ledger")
	if err != nil {
		return err
	}
	if err := checkFields("the ledger", top, ledgerSchema, ""); err != nil {
		return err
	}
	var claims []json.RawMessage
	if err := json.Unmarshal(top["narrative_claims"], &claims); err != nil {
		return fmt.Errorf("narrative_claims: %w", err)
	}
	for i, c := range claims {
		where := fmt.Sprintf("narrative_claims[%d]", i)
		fields, err := readObject(c, where)
		if err != nil {
			return err
		}
		if err := checkFields(where, fields, claimSchema, ""); err != nil {
			return err
		}
		if err := checkCheck(where+".check", fields["check"]); err != nil {
			return err
		}
	}
	return nil
}

// checkCheck checks a claim's check against the schema its type names.
func checkCheck(where string, raw json.RawMessage) error {
	fields, err := readObject(raw, where)
	if err != nil {
		return err
	}
	typ, ok := fields["type"]
	if !ok {
		return fmt.Errorf("%s.type is missing", where)
	}
	var name string
	_ = json.Unmarshal(typ, &name)
	sch, ok := checkSchemas[name]
	if !ok {
		return fmt.Errorf("%s.type is %s, want one of manual|present_artifact", where, typ)
	}
	return checkFields(where, fields, sch, fmt.Sprintf(" for type %q", name))
}

// readObject decodes data as one JSON object and returns its keys, failing on
// null, on any other non-object value, and on a key that appears twice. It
// reads every copy of every key, which json.Unmarshal into a map does not.
func readObject(data []byte, where string) (map[string]json.RawMessage, error) {
	dec := json.NewDecoder(bytes.NewReader(data))
	tok, err := dec.Token()
	if err != nil {
		return nil, fmt.Errorf("%s: %w", where, err)
	}
	if tok == nil {
		return nil, fmt.Errorf("%s is null", where)
	}
	if tok != json.Delim('{') {
		return nil, fmt.Errorf("%s is not an object", where)
	}
	fields := map[string]json.RawMessage{}
	for dec.More() {
		tok, err := dec.Token()
		if err != nil {
			return nil, fmt.Errorf("%s: %w", where, err)
		}
		key, ok := tok.(string)
		if !ok {
			return nil, fmt.Errorf("%s: unexpected token %v", where, tok)
		}
		if _, dup := fields[key]; dup {
			return nil, fmt.Errorf("%s has duplicate key %q", where, key)
		}
		var v json.RawMessage
		if err := dec.Decode(&v); err != nil {
			return nil, fmt.Errorf("%s: %w", where, err)
		}
		fields[key] = v
	}
	return fields, nil
}

// checkFields checks one object's keys against sch: unknown keys, then missing
// keys, then each present key for null and kind, each pass in sorted order.
// suffix follows an unknown-key fault, to name the check type that decides it.
func checkFields(where string, fields map[string]json.RawMessage, sch schema, suffix string) error {
	path := func(k string) string {
		if where == "the ledger" {
			return k
		}
		return where + "." + k
	}
	keys := sortedKeys(fields)
	for _, k := range keys {
		if _, ok := sch[k]; !ok {
			return fmt.Errorf("%s has unknown key %q%s", where, k, suffix)
		}
	}
	for _, k := range sortedKeys(sch) {
		if _, ok := fields[k]; sch[k].required && !ok {
			return fmt.Errorf("%s is missing", path(k))
		}
	}
	for _, k := range keys {
		if string(fields[k]) == "null" {
			return fmt.Errorf("%s is null", path(k))
		}
		if fault := sch[k].kind(fields[k]); fault != "" {
			return fmt.Errorf("%s %s", path(k), fault)
		}
	}
	return nil
}

func sortedKeys[V any](m map[string]V) []string {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	return keys
}

// danglingSymlink reports whether path exists as a directory entry but not
// once its symbolic links are followed.
func danglingSymlink(path string) bool {
	if _, err := os.Lstat(path); err != nil {
		return false
	}
	_, err := os.Stat(path)
	return errors.Is(err, fs.ErrNotExist)
}

// report renders the human-readable oracle report from a result.
func report(r result) string {
	var b strings.Builder
	line := strings.Repeat("=", 72)
	dash := strings.Repeat("-", 72)
	fmt.Fprintln(&b, line)
	fmt.Fprintln(&b, "CROSSCHECK CONFORMANCE / INVENTORY ORACLE")
	fmt.Fprintln(&b, line)
	fmt.Fprintf(&b, "skills discovered : %d\n", len(r.skills))
	fmt.Fprintf(&b, "agents discovered : %d\n", len(r.agents))
	fmt.Fprintf(&b, "docs scanned      : %s\n", pyList(r.presentDocs))
	fmt.Fprintf(&b, "referenced tokens : %d\n", len(r.refTokens))
	fmt.Fprintln(&b, dash)
	fmt.Fprintf(&b, "ERRORS   : %d\n", len(r.errors))
	for _, e := range r.errors {
		fmt.Fprintf(&b, "  ✗ %s\n", e)
	}
	fmt.Fprintf(&b, "WARNINGS : %d\n", len(r.warnings))
	for _, w := range r.warnings {
		fmt.Fprintf(&b, "  ⚠ %s\n", w)
	}
	fmt.Fprintln(&b, dash)
	fmt.Fprintf(&b, "NARRATIVE LEDGER (%d claims):\n", len(r.ledger))
	for _, c := range r.ledger {
		status := c.Status
		if status == "" {
			status = "?"
		}
		fmt.Fprintf(&b, "  • %s [%s]  %s\n", c.ID, status, c.Claim)
		fmt.Fprintf(&b, "      reality: %s\n", c.Reality)
		if c.TrackedIn != "" {
			fmt.Fprintf(&b, "      tracked: %s\n", c.TrackedIn)
		}
	}
	fmt.Fprintln(&b, line)
	res := "PASS"
	if len(r.errors) > 0 {
		res = "FAIL"
	}
	fmt.Fprintf(&b, "RESULT: %s\n", res)
	return b.String()
}

// defaultRoot resolves the scanned root: an explicit override arg, else the
// "crosscheck" plugin dir relative to the current working directory. The oracle
// is designed to be invoked from the repo root (`go run ./crosscheck/conformance`),
// so the bare default points at the plugin dir there. If that directory is not
// present (e.g. invoked from inside the plugin dir itself), it falls back to ".".
func defaultRoot(args []string) string {
	if len(args) > 1 {
		return args[1]
	}
	if fileExists("crosscheck") {
		return "crosscheck"
	}
	return "."
}

func main() {
	root := defaultRoot(os.Args)
	r := analyze(root)
	fmt.Print(report(r))
	if len(r.errors) > 0 {
		os.Exit(1)
	}
}
