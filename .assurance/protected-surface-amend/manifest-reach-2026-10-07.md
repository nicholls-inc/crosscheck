> **Action needed: Resolve markers and checklist before merging**
> You are being asked to verify this governance-note block as PR reviewer because the edit touches a protected surface. Approving means the protected-surface edit merges as governed and traceable; declining means the author must resolve every `REQUIRES HUMAN VERIFICATION:` marker and Review Checklist item first. Full explanation: https://github.com/nicholls-inc/crosscheck/blob/main/docs/gates/protected-surface-amendment.md.

## Protected-Surface Amendment

**Target file(s):** `cgv/prover/scripts/ProtectedStatements.lean`, `cgv/prover/protected-statements.txt`, `.github/workflows/cgv-ci.yml`, `.claude/rules/protected-surfaces.md`
**Class:** CGV proof surface (the manifest and its generator); A (CI enforcement; harness rules)
**Matched rule:** `cgv/prover/scripts/ProtectedStatements.lean`, `cgv/prover/protected-statements.txt`, `.github/workflows/**`, `.claude/rules/**`
**Date:** 2026-10-07

### Change Description

1. `cgv/prover/scripts/ProtectedStatements.lean`: the walk that decides which definitions the manifest hashes now starts from every constant used in a protected theorem's statement, as well as from `constraintImplies`, `IsDataPath` and `stepwiseSound` (SM-1, amended). It scopes by defining module, not by namespace, so private helpers count. The generator also reads the CGV table in `.claude/rules/protected-surfaces.md` and fails if the table and `protectedTheorems` / `protectedDefinitions` differ in names or files (SM-9).
2. `cgv/prover/protected-statements.txt`: regenerated. It gains 279 blocks, among them `CheckResult.isError`, `ResultEntry.severity`, `runChecker`, `checkPath`, `enumeratePaths`, `closedStates`, `searchSetup`, `checkHop` and `incompleteWith`. No theorem statement and none of the 107 earlier blocks changes. The section heading changes.
3. `.github/workflows/cgv-ci.yml`: runs on changes to `.claude/rules/protected-surfaces.md` (CI-1, amended), and runs the new self-test `cgv/prover/scripts/manifest-selftest.sh` after the manifest step (SM-11).
4. `.claude/rules/protected-surfaces.md`: the description of what the manifest records, one sentence saying a checker change now changes it, and one saying the generator checks the table and CGV CI runs when the file changes.

### Rationale

Task TB-1.3, issue #51, found in the review of #46 (https://github.com/nicholls-inc/crosscheck/pull/46#discussion_r4136326001 and #discussion_r4136326020). A redefinition of `CheckResult.isError` could make `*_noErrors` promise almost nothing without changing the manifest, and a theorem added to the rules table but not to the generator was never recorded. Intent: `intent/2026-10-07-manifest-reach.md`. Spec: `intent/2026-10-07-manifest-reach-spec.md`. Plan: `intent/2026-10-07-manifest-reach-plan.md`.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (item TB-1)
- **Title:** Shrink the trusted base, and measure what remains
- **Scope coverage:** TB-1 lists issue #51. Task TB-1.3 in `docs/TASKS.md` is this change.

### Authority

- **Authoriser:** harry-nicholls. The merge is the approval.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `cgv/prover/scripts/ProtectedStatements.lean` | header comment; `inScope`; new `roots`, `backticked`, `parseRulesTable`, `checkRulesTable`; `reach`; `render`; `main` | roots widened, scope by module, table check added |
| 2 | `cgv/prover/protected-statements.txt` | whole file | regenerated; 279 blocks added, section heading reworded |
| 3 | `.github/workflows/cgv-ci.yml` | `on.pull_request.paths`, `on.push.paths`; new step after the manifest step | trigger added; step added |
| 4 | `.claude/rules/protected-surfaces.md` | CGV proof surfaces: the paragraph after the table, and the manifest paragraph | reworded to the wider reach; table check and checker-change sentences added |

### Test / Coverage Impact

- `cgv/prover/scripts/manifest-selftest.sh` (SM-10) runs ten cases against the real sources: the unedited sources give the committed manifest; five doctored copies of the rules table each make the generator fail with the expected message; a change to `incompleteWith`, which only a statement reaches, and to a private helper it calls each change that constant's manifest block; a change to the unreached `outputToJson` and a change to `checkPath`'s termination proof each leave the manifest unchanged. Each part of the change was mutated in turn, and a case failed each time (see the PR body).
- No theorem statement, no definition, and no invariant changes. Exit 0 promises the same. More changes now need protected-surface review: every change to a definition the statements mention.

### Review Checklist

- [x] Rationale is anchored to issue #51 and the review threads of #46.
- [x] Authoriser is a named human.
- [x] TB-1 lists issue #51.
- [x] The diff plan names every changed protected file.
- [x] No check is weakened: the manifest only gains blocks, and the generator only gains failure modes.
- [ ] REQUIRES HUMAN VERIFICATION: the maintainer accepts that every change to the checker's definitions (`checkEdge`, `checkPath`, `runChecker` and what they reach) now changes the manifest and so needs Tier 3 review.
