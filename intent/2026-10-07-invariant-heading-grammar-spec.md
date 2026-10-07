# Spec: One invariant-heading grammar

Intent: `intent/2026-10-07-invariant-heading-grammar.md`. Task: PB-1.21. Tier: 3.

## Definitions

- **ID.** `I` followed by one or more digits and an optional lower-case letter. As a regular expression, `I\d+[a-z]?`.
- **Canonical heading.** A line matching `^## (I\d+[a-z]?):`.
- **Covering comment.** A line matching the template's comment pattern, `Invariant <ID>: ` after `//` (all three templates) or `#` (python only).
- **Invariant doc.** A file `docs/invariants/*.md` at any depth in the repository, outside `.git`, `node_modules`, `.claude`, and build output (`target`, `dist`, `.lake`).

## Requirements

**HG-1. One alphabet.** In each of `python-template.md`, `go-template.md` and `typescript-template.md` under `crosscheck/skills/invariant-coverage-scaffold/references/`, and in `crosscheck/docs/examples/workflows/tier-a/check_invariant_coverage.py`, the ID group of the comment pattern is `(I\d+[a-z]?)`, the same as the header pattern's.

**HG-2. Agreement is tested.** `heading_grammar_test.go` extracts each header pattern and each comment pattern from those four files. For every ID in a fixed list (`I1`, `I1a`, `I42`, `Q1`, `A1`, `IA1`, `INV1`, `i1`, `I`, `1`, `I1A`), it builds the header line `## <id>: Name` and the comment line for the file's language. The test fails unless the header pattern and the comment pattern either both match, capturing the same ID, or both do not match. It also fails if the comment pattern matches a comment with any ID that the canonical heading pattern rejects.

**HG-3. The header corpus covers the tier-a script.** The existing header agreement test also checks the `HEADER_RE` of `tier-a/check_invariant_coverage.py`.

**HG-4. Real docs are canonical.** For every invariant doc, the test fails when:
- a line that names an invariant is not a canonical heading. A line names an invariant when it matches `^\s*#{1,6}\s*\**[A-Za-z]+\d+[a-z]?\b` or `^\s*\*\*[A-Za-z]+\d+[a-z]?[.:]`. Lines inside a fenced code block are skipped;
- the doc has no canonical heading.

The test also fails when it finds no invariant doc at all, so a moved directory cannot turn it into a no-op.

**HG-5. The four docs migrate.** Each `### I<N> — <Name>` heading becomes `## I<N>: <Name>`, with the name unchanged. The `## Invariants` heading above them is removed, since the invariants are now its siblings.

**HG-6. The example migrates.** In `crosscheck/docs/examples/workflows/example.md` the queue invariants are `## I1: FIFO_ORDER`, `## I2: DEAD_LETTER_TERMINAL` and `## I3: NO_DUPLICATE_DELIVERY`, each followed by its prose. Every `Q1`, `Q2` and `Q3` in the example becomes `I1`, `I2` and `I3`. The description of the parser names `## I1:` headings, and the sample error's line number matches the excerpt.

## Behaviour that changes

A test comment whose ID prefix is not `I`, such as `# Invariant Q1: Name.`, is no longer read by the coverage gate. Before, it was read and reported as covered-but-undeclared. That report was always a failure, because no header pattern could declare `Q1`.

## Known gaps

- A comment with a prefix other than `I` is ignored silently. A lint that names it is not yet reached in this change. The property that blocks it is a scanner pass for near-miss comments, and the open question is whether a near miss should fail the gate or only warn. Row PB-1.26.
- `crosscheck/docs/examples/workflows/tier-b/assurance_pr_gate_plan.py` parses invariants with `^##+\s*Invariant\s+([A-Z0-9_-]+)`, a form no canonical doc uses, so it finds no invariant in a canonical doc. Row PB-1.27.
- HG-4 covers invariant docs in this repository. A user repository's docs are checked only by the gate the scaffold writes there.
