## Protected-Surface Amendment

**Target file(s):** `scripts/ci/incident-eval-check.mjs`, `scripts/ci/incident-eval-check.test.mjs`, `scripts/ci/tier-gate.mjs`, `scripts/ci/tier-gate.test.mjs`, `docs/assurance/TIER-LAYER-MAP.md`, `docs/assurance/DEVELOPMENT-FRAMEWORK.md`, `evals/README.md`
**Class:** A (CI enforcement, governance documents, evaluation suite)
**Matched rule:** `scripts/ci/**`, `docs/assurance/**`, `evals/**`
**Date:** 2026-10-06

### Change Description

1. `scripts/ci/incident-eval-check.mjs`: the failure message no longer says the change "stays blocked". It asks for the eval and the candidate invariant in a follow-up pull request and links to `docs/gates/incident-eval-check.md` instead of the gate index (IE-8). The empty-range comment names a merge commit and drops the rebase merge (IE-2). The trigger, the lookups and the exit codes do not change.
2. `scripts/ci/incident-eval-check.test.mjs`: the expected failure output follows IE-8. Two new tests simulate GitHub's rebase merge and show that the check reads the pull request's commits (IE-6).
3. `scripts/ci/tier-gate.mjs`: row 6 of `EVIDENCE_CLASSES`, `evals/**`, changes from checked by the Incident Eval Check to not yet reached, with a blocking property and an open question (TG-8 row 6). The gate's pass or fail result does not change.
4. `scripts/ci/tier-gate.test.mjs`: the every-class report test expects the new line.
5. `docs/assurance/TIER-LAYER-MAP.md`: the evidence table and its list of blocking properties follow TG-8 row 6.
6. `docs/assurance/DEVELOPMENT-FRAMEWORK.md`: the stage table and stage 6 name the candidate invariant, and stage 5 links to the new explainer instead of saying it does not exist.
7. `evals/README.md`: "CI linkage" asks for the candidate invariant in every case, names both invariant directories, and says the check runs after the merge and cannot block it.

### Rationale

Task PB-1.12. PB-1.8 made stage 5 of `DEVELOPMENT-FRAMEWORK.md` describe the check as the code behaves. These surfaces still described it the old way: a failure message that claims to block a merge it prints after, an evidence row that credits the check with testing evals it never runs, a README that makes the invariant optional, and a spec line (IE-2) that says a GitHub rebase merge leaves an empty range, when GitHub's documentation says a rebase merge "always updates the committer information and creates new commit SHAs". Intent: `intent/2026-10-06-incident-eval-surfaces.md`. Spec: `intent/2026-10-06-incident-eval-surfaces-spec.md`. Plan: `intent/2026-10-06-incident-eval-surfaces-plan.md`.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (immediate horizon, item PB-1)
- **Title:** Adopt the AI-native SDLC playbook as the development framework and make every human gate self-explanatory
- **Scope coverage:** PB-1 names `incident-eval-check.yml` as one of its CI jobs, and asks that every gate name its explainer under `docs/gates/`, reached from a fixed three-sentence gate message. Task PB-1.12 in `docs/TASKS.md` is this change.

### Authority

- **Authoriser:** harry-nicholls. The merge is the approval.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `scripts/ci/incident-eval-check.mjs` | `GATE_DOC`, `printFailure`, empty-range comment | changed |
| 2 | `scripts/ci/incident-eval-check.test.mjs` | `failureOutput`, rebase-merge helper and tests | changed, added |
| 3 | `scripts/ci/tier-gate.mjs` | `EVIDENCE_CLASSES` row 6 | changed |
| 4 | `scripts/ci/tier-gate.test.mjs` | every-class report test | one expected line changed |
| 5 | `docs/assurance/TIER-LAYER-MAP.md` | "Evidence and sign-off" | row and bullet changed |
| 6 | `docs/assurance/DEVELOPMENT-FRAMEWORK.md` | stage table, stage 5, stage 6 | changed |
| 7 | `evals/README.md` | "CI linkage" | rewritten |

### Test / Coverage Impact

- `node --test scripts/ci/*.test.mjs` gains the two IE-6 rebase-merge cases. Every exit-1 case asserts the whole IE-8 output.
- The tier gate's report for `evals/**` now says "not yet reached". This narrows what the report claims as evidence. It weakens no check: the Incident Eval Check never ran an eval, and the gate's result does not depend on the report.
- No invariant or eval changes.

### Review Checklist

- [x] Rationale is anchored to the script, the workflow trigger and GitHub's merge-method documentation.
- [x] Authoriser is a named human.
- [x] PB-1 covers `incident-eval-check.yml`, the tier gate and the gate explainers.
- [x] The diff plan names every changed protected file.
- [x] No check is weakened. The check's trigger and exit codes, and the tier gate's result, do not change.
- [ ] REQUIRES HUMAN VERIFICATION: The maintainer accepts that the rebase-merge tests simulate GitHub's rebase merge locally, because the ruleset forbids rebase merges, and that a follow-up pull request with no incident reference of its own leaves the original red run red (spec, "Concerns flagged").
