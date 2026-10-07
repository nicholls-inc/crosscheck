# Plan: One invariant-heading grammar

Intent: `intent/2026-10-07-invariant-heading-grammar.md`
Spec: `intent/2026-10-07-invariant-heading-grammar-spec.md` (HG-1 to HG-6)
Governing roadmap item: PB-1. Task: PB-1.21. Tier: 3.

This plan is not the root `plan.md`, which belongs to an earlier change. The pull request body cites this file with a `Plan:` line.

## Order of work

1. Write the governance note `.assurance/protected-surface-amend/invariant-heading-grammar-2026-10-07.md`, naming the three protected invariant docs.
2. Extend `crosscheck/conformance/heading_grammar_test.go`: add the tier-a script to the header cases (HG-3), add `TestCommentGrammarAgreement` (HG-2) and `TestRealInvariantDocsCanonical` (HG-4). Run `go test ./...` in `crosscheck/conformance` and see both new tests fail on the unchanged tree. Commit the tests on their own.
3. Narrow the comment pattern's ID group to `(I\d+[a-z]?)` in the three templates and the tier-a script (HG-1).
4. Migrate the four invariant docs (HG-5) and the worked example (HG-6).
5. Run `go vet ./... && go test ./... && go run . ..` in `crosscheck/conformance`. Mutate to check the tests bite: put `[A-Z]+` back in one template, and put one doc heading back to `### I1 —`. Each mutant must fail a test. Restore with git.
6. Set PB-1.21 to `done` in `docs/TASKS.md` with the intent as its record. Add rows PB-1.26 (near-miss comment lint) and PB-1.27 (tier-b parser grammar). Add a `crosscheck/JOURNAL.md` entry.

## Files

| File | Protected | Change |
|---|---|---|
| `crosscheck/docs/invariants/extractDifficultyMetrics.md` | yes (Class B) | headings to h2 (HG-5) |
| `crosscheck/docs/invariants/parseDafnyOutput.md` | yes (Class B) | same |
| `crosscheck/docs/invariants/shouldExclude.md` | yes (Class B) | same |
| `crosscheck/skills/journal-context/docs/invariants/journal-context.md` | no glob matches | same |
| `crosscheck/skills/invariant-coverage-scaffold/references/{python,go,typescript}-template.md` | no | comment ID group (HG-1) |
| `crosscheck/docs/examples/workflows/tier-a/check_invariant_coverage.py` | no | comment ID group (HG-1) |
| `crosscheck/docs/examples/workflows/example.md` | no | `## I<N>:` headings, `I` IDs (HG-6) |
| `crosscheck/conformance/heading_grammar_test.go` | no | HG-2, HG-3, HG-4 |
| `docs/TASKS.md`, `crosscheck/JOURNAL.md` | no | record |
| `.assurance/protected-surface-amend/invariant-heading-grammar-2026-10-07.md` | no | new governance note |
| `intent/2026-10-07-invariant-heading-grammar*.md` | no | stage artefacts |

## Risks

- **A user repository that tagged tests `Invariant Q1:`** loses those comments from the scan. Before, each one failed the gate as covered-but-undeclared, so no passing gate starts failing. A gate that failed on them now passes if no other fault remains. PB-1.26 tracks a lint for this.
- **The doc walk in HG-4 reaches into a large tree.** It skips `.git`, `node_modules`, `.claude` and build output, so a local checkout with worktrees under `.claude/worktrees` is not walked.
- **Readers lose the `## Invariants` section heading.** The `## Purpose` and `## Carve-outs` sections stay, and the invariants sit between them as h2 siblings, as the `/assurance-init` skeleton lays them out.

## Proof that it worked

- `go test ./...` in `crosscheck/conformance` passes, and the new tests fail on the tree before steps 3 and 4.
- `grep -cE '^## I[0-9]+[a-z]?:'` prints 5, 7, 6 and 7 for the four docs.
- Each mutant in step 5 fails a named test.
- This pull request's Tier Gate run passes at Tier 3.
