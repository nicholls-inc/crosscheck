package main

// Cross-toolchain guard for the ADD invariant-heading grammar (issue #227).
//
// Three shipped artifacts independently parse or gate on the invariant-heading
// form, and they used to disagree: the add-orchestrator quality gate required
// h2 `## I<N>:`, while the invariant-coverage-scaffold templates parsed the
// legacy bold-prefix `**I1. Name.**`. A module that passed one failed the
// other. The conformance oracle did not catch it — it checks reference and
// inventory integrity, not semantic agreement between two parsers.
//
// This guard pins every parser/gate to ONE canonical grammar by extracting the
// real pattern each artifact ships and running a shared corpus of headings
// through all of them. If any artifact's pattern drifts, the corpus stops
// agreeing and this test fails. It is deliberately extraction-based, not a
// string-literal match, so it survives reformatting but still bites on a
// genuine grammar change.
//
// Canonical grammar (skills/draft-invariants/SKILL.md Step 3): an h2 heading
// `## I<N>: <Name>`, where <N> is digits with an optional lowercase
// sub-invariant suffix (I1, I1a). The covering test comment
// `Invariant I<N>: <Name>` uses the same ID alphabet (PB-1.21). If you change
// this, change it in:
//   - agents/add-orchestrator.md Step 6 quality-gate grep
//   - skills/invariant-coverage-scaffold/references/{python,go,typescript}-template.md HEADER_RE and COMMENT_RE
//   - docs/examples/workflows/tier-a/check_invariant_coverage.py HEADER_RE and COMMENT_RE
//   - skills/assurance-init/SKILL.md emitted invariant-doc template
//   - skills/draft-invariants/SKILL.md Step 3 heading convention
//   - every docs/invariants/*.md in the repository
// ...and this test, which will fail until they agree again.

import (
	"bufio"
	"bytes"
	"io/fs"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"testing"
)

// canonicalHeading is the single source of truth this guard pins everything to.
var canonicalHeading = regexp.MustCompile(`^## (I\d+[a-z]?):`)

// headingCorpus is the shared accept/reject set. Every shipped pattern must
// classify each line exactly as canonicalHeading does.
var headingCorpus = []struct {
	line string
	want bool
}{
	{"## I1: RedactionSentinelString", true},
	{"## I7: BothSetFileWins", true},
	{"## I42: Foo", true},
	{"## I1a: SubInvariant", true}, // optional lowercase sub-invariant suffix
	{"### I1: Name", false},        // h3, not h2
	{"#### I1: x", false},          // h4
	{"#I1: nospace", false},        // not an h2 heading
	{" ## I1: leadingspace", false},
	{"**I1. RedactionSentinelString.**", false},           // legacy bold-prefix form
	{"## Invariants", false},                              // section header, not an invariant
	{"## Engine selection — single embedding API", false}, // prose-section heading
	{"## A1: WrongPrefix", false},                         // non-I prefix
	{"## i1: lowercasePrefix", false},                     // lowercase prefix
	{"I1: not a heading", false},
	{"**I1:** inline reference, not a heading", false},
}

// crosscheckRoot resolves the plugin root regardless of whether the test binary
// runs from crosscheck/conformance (the usual `go test ./...` case) or the repo
// root.
func crosscheckRoot(t *testing.T) string {
	t.Helper()
	for _, cand := range []string{"..", "crosscheck", "."} {
		if _, err := os.Stat(filepath.Join(cand, "agents", "add-orchestrator.md")); err == nil {
			return cand
		}
	}
	t.Fatalf("could not locate the crosscheck plugin root (agents/add-orchestrator.md not found from %q)", mustGetwd(t))
	return ""
}

func mustGetwd(t *testing.T) string {
	t.Helper()
	wd, _ := os.Getwd()
	return wd
}

// extract pulls the first capture group of locator out of the named file,
// failing loudly if the anchor has moved (a rename should trip this guard, not
// silently skip it).
func extract(t *testing.T, root, rel string, locator *regexp.Regexp) string {
	t.Helper()
	path := filepath.Join(root, rel)
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read %s: %v", path, err)
	}
	m := locator.FindSubmatch(data)
	if m == nil {
		t.Fatalf("could not locate the heading pattern in %s via /%s/ — if you renamed or reformatted it, update this guard", rel, locator)
	}
	return string(m[1])
}

// assertAgrees compiles a shipped pattern and checks it classifies the corpus
// exactly as the canonical grammar does.
func assertAgrees(t *testing.T, source, pattern string) {
	t.Helper()
	re, err := regexp.Compile(pattern)
	if err != nil {
		t.Fatalf("%s: shipped pattern %q does not compile as a Go regexp: %v", source, pattern, err)
	}
	for _, c := range headingCorpus {
		got := re.MatchString(c.line)
		if got != c.want {
			t.Errorf("%s pattern %q disagrees with canonical on %q: got match=%v, want %v",
				source, pattern, c.line, got, c.want)
		}
	}
}

// TestCanonicalGrammarSelfConsistent is a sanity check that the corpus matches
// the canonical regex as annotated — guards against a typo in the corpus itself.
func TestCanonicalGrammarSelfConsistent(t *testing.T) {
	for _, c := range headingCorpus {
		if got := canonicalHeading.MatchString(c.line); got != c.want {
			t.Errorf("corpus annotation wrong for %q: canonical match=%v, annotated want=%v", c.line, got, c.want)
		}
	}
}

// TestHeadingGrammarAgreement is the cross-toolchain guard: the orchestrator
// gate grep and all three coverage-scaffold parsers must accept the same
// heading grammar.
func TestHeadingGrammarAgreement(t *testing.T) {
	root := crosscheckRoot(t)

	cases := []struct {
		source  string
		rel     string
		locator *regexp.Regexp
	}{
		{
			source:  "add-orchestrator Step 6 grep",
			rel:     filepath.Join("agents", "add-orchestrator.md"),
			locator: regexp.MustCompile("grep -cE '([^']*)'"),
		},
		{
			source:  "python-template HEADER_RE",
			rel:     filepath.Join("skills", "invariant-coverage-scaffold", "references", "python-template.md"),
			locator: regexp.MustCompile(`HEADER_RE = re\.compile\(r"([^"]*)"\)`),
		},
		{
			source:  "go-template headerRe",
			rel:     filepath.Join("skills", "invariant-coverage-scaffold", "references", "go-template.md"),
			locator: regexp.MustCompile("headerRe\\s*=\\s*regexp\\.MustCompile\\(`([^`]*)`\\)"),
		},
		{
			source:  "typescript-template HEADER_RE",
			rel:     filepath.Join("skills", "invariant-coverage-scaffold", "references", "typescript-template.md"),
			locator: regexp.MustCompile(`const HEADER_RE = /([^/]*)/;`),
		},
		{
			source:  "tier-a check_invariant_coverage.py HEADER_RE",
			rel:     filepath.Join("docs", "examples", "workflows", "tier-a", "check_invariant_coverage.py"),
			locator: regexp.MustCompile(`HEADER_RE = re\.compile\(r"([^"]*)"\)`),
		},
	}

	for _, c := range cases {
		pattern := extract(t, root, c.rel, c.locator)
		assertAgrees(t, c.source, pattern)
	}
}

// TestAssuranceInitEmitsCanonical checks that the skeleton /assurance-init
// writes uses canonical h2 invariant headings and no legacy bold-prefix emit.
func TestAssuranceInitEmitsCanonical(t *testing.T) {
	root := crosscheckRoot(t)
	path := filepath.Join(root, "skills", "assurance-init", "SKILL.md")
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read %s: %v", path, err)
	}
	multiCanonical := regexp.MustCompile(`(?m)^## I\d+[a-z]?:`)
	if n := len(multiCanonical.FindAll(data, -1)); n < 2 {
		t.Errorf("assurance-init SKILL.md should emit >=2 canonical h2 invariant headings, found %d", n)
	}
	legacyEmit := regexp.MustCompile(`(?m)^\*\*I\d+\.\s`)
	if legacyEmit.Match(data) {
		t.Errorf("assurance-init SKILL.md still emits a legacy bold-prefix invariant heading (**I<N>. ...); migrate it to '## I<N>: <Name>'")
	}
}

// commentGrammarSources are the shipped coverage scanners. Each pairs the
// header pattern that declares an ID with the comment pattern that covers it.
var commentGrammarSources = []struct {
	source  string
	rel     string
	header  *regexp.Regexp
	comment *regexp.Regexp
	marker  string // how a comment starts in the file's language
}{
	{
		source:  "python-template",
		rel:     filepath.Join("skills", "invariant-coverage-scaffold", "references", "python-template.md"),
		header:  regexp.MustCompile(`HEADER_RE = re\.compile\(r"([^"]*)"\)`),
		comment: regexp.MustCompile(`COMMENT_RE = re\.compile\(r"([^"]*)"\)`),
		marker:  "#",
	},
	{
		source:  "go-template",
		rel:     filepath.Join("skills", "invariant-coverage-scaffold", "references", "go-template.md"),
		header:  regexp.MustCompile("headerRe\\s*=\\s*regexp\\.MustCompile\\(`([^`]*)`\\)"),
		comment: regexp.MustCompile("commentRe\\s*=\\s*regexp\\.MustCompile\\(`([^`]*)`\\)"),
		marker:  "//",
	},
	{
		source:  "typescript-template",
		rel:     filepath.Join("skills", "invariant-coverage-scaffold", "references", "typescript-template.md"),
		header:  regexp.MustCompile(`const HEADER_RE = /([^/]*)/;`),
		comment: regexp.MustCompile(`(?m)^const COMMENT_RE = /(.*)/;\s*$`),
		marker:  "//",
	},
	{
		source:  "tier-a check_invariant_coverage.py",
		rel:     filepath.Join("docs", "examples", "workflows", "tier-a", "check_invariant_coverage.py"),
		header:  regexp.MustCompile(`HEADER_RE = re\.compile\(r"([^"]*)"\)`),
		comment: regexp.MustCompile(`COMMENT_RE = re\.compile\(r"([^"]*)"\)`),
		marker:  "#",
	},
}

// idCorpus mixes canonical IDs with near misses: another prefix, a longer
// prefix, a lowercase prefix, no digits, no prefix, an upper-case suffix.
var idCorpus = []string{"I1", "I1a", "I42", "Q1", "A1", "IA1", "INV1", "i1", "I", "1", "I1A"}

// TestCommentGrammarAgreement pins the two halves of each coverage gate to one
// ID alphabet (HG-2). A comment pattern that reads an ID the header pattern
// cannot declare turns every such comment into a covered-but-undeclared
// failure; one that misses an ID the header declares hides real coverage.
func TestCommentGrammarAgreement(t *testing.T) {
	root := crosscheckRoot(t)
	for _, src := range commentGrammarSources {
		header := compileShipped(t, src.source+" header", extract(t, root, src.rel, src.header))
		comment := compileShipped(t, src.source+" comment", extract(t, root, src.rel, src.comment))

		if comment.FindStringSubmatch(src.marker+" Invariant I1: Name.") == nil {
			t.Errorf("%s comment pattern %q does not read the canonical comment %q", src.source, comment, src.marker+" Invariant I1: Name.")
		}
		for _, id := range idCorpus {
			headerLine := "## " + id + ": Name"
			commentLine := src.marker + " Invariant " + id + ": Name."
			h := header.FindStringSubmatch(headerLine)
			c := comment.FindStringSubmatch(commentLine)
			switch {
			case (h == nil) != (c == nil):
				t.Errorf("%s: header %q matches %q = %v, but comment %q matches %q = %v",
					src.source, header, headerLine, h != nil, comment, commentLine, c != nil)
			case h != nil && h[1] != c[1]:
				t.Errorf("%s: header reads ID %q from %q, comment reads %q from %q", src.source, h[1], headerLine, c[1], commentLine)
			}
			if c != nil && !canonicalHeading.MatchString(headerLine) {
				t.Errorf("%s: comment pattern %q reads ID %q, which no canonical heading can declare", src.source, comment, id)
			}
		}
	}
}

func compileShipped(t *testing.T, source, pattern string) *regexp.Regexp {
	t.Helper()
	re, err := regexp.Compile(pattern)
	if err != nil {
		t.Fatalf("%s: shipped pattern %q does not compile as a Go regexp: %v", source, pattern, err)
	}
	return re
}

// namesInvariant finds a line that declares an invariant in any form: an
// ATX heading or a bold paragraph lead whose first word is a prefix and a
// number (### I1, ## Q2:, **I3.).
var namesInvariant = []*regexp.Regexp{
	regexp.MustCompile(`^\s*#{1,6}\s*\**[A-Za-z]+\d+[a-z]?\b`),
	regexp.MustCompile(`^\s*\*\*[A-Za-z]+\d+[a-z]?[.:]`),
}

var skipDirs = map[string]bool{".git": true, "node_modules": true, ".claude": true, "target": true, "dist": true, ".lake": true}

// invariantDocs walks the repository for docs/invariants/*.md at any depth.
func invariantDocs(t *testing.T) (string, []string) {
	t.Helper()
	root, err := filepath.Abs(crosscheckRoot(t))
	if err != nil {
		t.Fatal(err)
	}
	repo := filepath.Dir(root)
	var docs []string
	err = filepath.WalkDir(repo, func(path string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if d.IsDir() {
			if path != repo && skipDirs[d.Name()] {
				return filepath.SkipDir
			}
			return nil
		}
		dir := filepath.Dir(path)
		if filepath.Ext(path) == ".md" && filepath.Base(dir) == "invariants" && filepath.Base(filepath.Dir(dir)) == "docs" {
			docs = append(docs, path)
		}
		return nil
	})
	if err != nil {
		t.Fatalf("walk %s: %v", repo, err)
	}
	return repo, docs
}

// TestRealInvariantDocsCanonical reads every invariant doc in the repository
// (HG-4). A doc whose invariants the gate grep cannot see passes every
// synthetic check above while failing the gate in practice.
func TestRealInvariantDocsCanonical(t *testing.T) {
	repo, docs := invariantDocs(t)
	if len(docs) == 0 {
		t.Fatalf("found no docs/invariants/*.md under %s; if they moved, update invariantDocs", repo)
	}
	for _, doc := range docs {
		rel, _ := filepath.Rel(repo, doc)
		data, err := os.ReadFile(doc)
		if err != nil {
			t.Fatalf("read %s: %v", rel, err)
		}
		canonical := 0
		inFence := false
		sc := bufio.NewScanner(bytes.NewReader(data))
		for n := 1; sc.Scan(); n++ {
			line := sc.Text()
			if trimmed := strings.TrimSpace(line); strings.HasPrefix(trimmed, "```") || strings.HasPrefix(trimmed, "~~~") {
				inFence = !inFence
				continue
			}
			if inFence {
				continue
			}
			if canonicalHeading.MatchString(line) {
				canonical++
				continue
			}
			for _, re := range namesInvariant {
				if re.MatchString(line) {
					t.Errorf("%s:%d names an invariant in a non-canonical form %q; use '## I<N>: <Name>'", rel, n, line)
					break
				}
			}
		}
		if canonical == 0 {
			t.Errorf("%s declares no canonical '## I<N>: <Name>' heading", rel)
		}
	}
}
