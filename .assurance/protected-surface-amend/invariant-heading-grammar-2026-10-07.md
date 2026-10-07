## Protected-Surface Amendment

**Target file(s):** `crosscheck/docs/invariants/extractDifficultyMetrics.md`, `crosscheck/docs/invariants/parseDafnyOutput.md`, `crosscheck/docs/invariants/shouldExclude.md`
**Class:** B (module invariant specifications)
**Matched rule:** `crosscheck/docs/invariants/**`
**Date:** 2026-10-07

`crosscheck/skills/journal-context/docs/invariants/journal-context.md` gets the same edit. No glob in `.claude/rules/protected-surfaces.md` matches it, but it is an invariant doc, so it is listed here as well.

### Change Description

1. In each of the three protected docs and in `journal-context.md`, every heading `### I<N> — <Name>` becomes `## I<N>: <Name>`, with `<Name>` unchanged. The `## Invariants` heading above them is removed. No statement, formula, covering-test line or carve-out changes.

### Rationale

Task PB-1.21, issue #26. The `add-orchestrator` quality gate counts invariants with `grep -cE '^## I[0-9]+[a-z]?:'`, and the coverage-gate templates use the same header pattern. Measured on `origin/main` (`7cafa0d`), the grep prints 0 for each of the four docs, so every real invariant doc in the repository fails the gate that `add-orchestrator.md:335` describes. The canonical form is the one `crosscheck/skills/draft-invariants/SKILL.md` Step 3 states. Intent: `intent/2026-10-07-invariant-heading-grammar.md`. Spec: `intent/2026-10-07-invariant-heading-grammar-spec.md` (HG-4, HG-5). Plan: `intent/2026-10-07-invariant-heading-grammar-plan.md`.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (immediate horizon, item PB-1)
- **Title:** Adopt the AI-native SDLC playbook as the development framework and make every human gate self-explanatory
- **Scope coverage:** Task PB-1.21 in `docs/TASKS.md`, refined from #26 in `intent/2026-10-07-backlog-review-decisions.md`, is this change.

### Authority

- **Authoriser:** harry-nicholls. The merge is the approval.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `crosscheck/docs/invariants/extractDifficultyMetrics.md` | `## Invariants`, I1 to I5 headings | h3 to h2 `## I<N>:`, parent heading removed |
| 2 | `crosscheck/docs/invariants/parseDafnyOutput.md` | `## Invariants`, I1 to I7 headings | same |
| 3 | `crosscheck/docs/invariants/shouldExclude.md` | `## Invariants`, I1 to I6 headings | same |

### Test / Coverage Impact

- `crosscheck/conformance/heading_grammar_test.go` gains `TestRealInvariantDocsCanonical` (HG-4), which reads these docs. It fails on `origin/main` and passes after the migration.
- No property test changes. The covering tests named in each doc are untouched, and no invariant is added, removed or reworded.
- The intent-check attestation in `crosscheck/mcp-server/.assurance/intent-check-attestation.json` hashes two of these docs. It is an LLM verdict that no gate reads, so it is not regenerated.

### Review Checklist

- [x] Rationale is anchored to a measured grep on `origin/main`.
- [x] Authoriser is a named human.
- [x] PB-1.21 governs the change.
- [x] The diff plan names every changed protected file.
- [x] No invariant is weakened: only heading lines change.
- [ ] REQUIRES HUMAN VERIFICATION: the maintainer accepts dropping the `## Invariants` parent heading in favour of h2 invariant headings.
