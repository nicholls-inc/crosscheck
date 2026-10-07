# Intent: One invariant-heading grammar for the gate, the templates and the real docs

Task: PB-1.21. Governing roadmap item: PB-1. Issue: #26. Decision: `intent/2026-10-07-backlog-review-decisions.md`, section "#26 → PB-1.21".

## Problem statement
The invariant-coverage gate has two halves. One scans invariant docs for headings, the other scans tests for `Invariant <ID>:` comments. The two disagree on which IDs exist.

- In the python, go and typescript templates of `/invariant-coverage-scaffold`, the header pattern accepts only the prefix `I` (`^## (I\d+[a-z]?):`). The comment pattern accepts any upper-case prefix (`[A-Z]+\d+[a-z]?`). The tier-a example script `crosscheck/docs/examples/workflows/tier-a/check_invariant_coverage.py` has the same split. A comment `# Invariant Q1:` is read as an ID that no header can ever declare.
- Every real invariant doc in the repository uses an h3 heading, `### I1 — Name`. The `add-orchestrator` quality gate (`grep -cE '^## I[0-9]+[a-z]?:'`) counts zero headings in each of them, and `add-orchestrator.md:335` says that form fails the gate. The four docs are `crosscheck/docs/invariants/extractDifficultyMetrics.md`, `parseDafnyOutput.md`, `shouldExclude.md` and `crosscheck/skills/journal-context/docs/invariants/journal-context.md`.
- `crosscheck/conformance/heading_grammar_test.go` runs a synthetic corpus through the header patterns only. It never reads a real invariant doc and never extracts a comment pattern, so it passes while both faults above are live.
- The worked example `crosscheck/docs/examples/workflows/example.md` declares invariants as `**Q1. FIFO_ORDER.**` and says the parser scans for `**Q1.`. No shipped parser accepts that form.

Measured on `origin/main` (`7cafa0d`): `grep -cE '^## I[0-9]+[a-z]?:'` prints 0 for each of the four docs.

## Proposed outcome
- One alphabet: the prefix is `I`. The header pattern, the comment pattern and the `add-orchestrator` grep all accept `I<digits>` with an optional lower-case suffix, and nothing else.
- The four real docs use the canonical h2 form `## I<N>: <Name>`, and the grep counts every invariant in them.
- The guard test reads every `docs/invariants/*.md` file in the repository and fails on a heading that names an invariant in any other form. It extracts each shipped comment pattern and fails when a comment pattern and its header pattern disagree on an ID.
- The worked example uses `## I<N>: <Name>` headings and `Invariant I<N>:` comments, and describes the parser as it is.

## Affected users and systems
- Anyone who copies a coverage-gate template. A test comment with a prefix other than `I` is no longer counted. Before, it was counted and reported as covered-but-undeclared, because no header could declare it.
- Readers of the four invariant docs. The content does not change, only the heading lines.
- `crosscheck/conformance/heading_grammar_test.go`, which runs in the `conformance` job of `.github/workflows/ci.yml`.
- `add-orchestrator.md` does not change. Its grep already accepts only `I`.

## Constraints
- The three `crosscheck/docs/invariants/*.md` files are Class B protected surfaces. The edit needs a governance note and Tier 3.
- The canonical form is the one `crosscheck/skills/draft-invariants/SKILL.md` Step 3 already states. No `SKILL.md` or agent file changes.
- No invariant's statement, test sketch or covering-test line changes. Only heading lines move.

## Open questions
None. The decision record says to pick one alphabet. The header, the grep and the draft skill already use `I`, so the comment pattern narrows to match them. Widening the header instead would change the canonical grammar that three shipped artefacts and `draft-invariants` state.

Two gaps stay open and get their own rows. A comment with a prefix other than `I` is now ignored silently, where a lint could name it. The tier-b example `assurance_pr_gate_plan.py` parses a third form, `## Invariant <ID>:`, which matches no canonical heading.
