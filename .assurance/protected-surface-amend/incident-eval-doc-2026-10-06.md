## Protected-Surface Amendment

**Target file(s):** `docs/assurance/DEVELOPMENT-FRAMEWORK.md`
**Class:** A (governance documents)
**Matched rule:** `docs/assurance/**`
**Date:** 2026-10-06

### Change Description

1. `docs/assurance/DEVELOPMENT-FRAMEWORK.md`: stage 5 replaces the `incident-eval-check.yml` bullet. The old bullet said the check fails when an incident record under `evals/` has no eval. The new bullet names the trigger (the `incident` label, or a `Fixes-Incident:` line in the body or a commit), the two artefacts it needs (an eval and a candidate invariant, each matched by substring), exit 1, exit 2, and that the workflow runs after the merge and cannot block it. It states that exit 2 can happen on any merged pull request, because the check reads the commits before it looks for an incident reference, and that the check has no explainer yet.

### Rationale

Task PB-1.8. The old bullet described a check that does not exist. `scripts/ci/incident-eval-check.mjs` never looks for incident records. It starts from the pull request's label, body and commits, needs a candidate invariant as well as an eval, and exits 2 when it cannot read the commits. `.github/workflows/incident-eval-check.yml` runs on `pull_request` `closed` with `merged == true`, so the check reports after the merge. #61 merged at 17:59:08 UTC on 2026-10-06 and its Incident Eval Check run started at 17:59:12; the run for #60, closed without a merge, was skipped. Intent: `intent/2026-10-06-incident-eval-doc.md`. Plan: `intent/2026-10-06-incident-eval-doc-plan.md`.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (immediate horizon, item PB-1)
- **Title:** Adopt the AI-native SDLC playbook as the development framework and make every human gate self-explanatory
- **Scope coverage:** PB-1's scope names `incident-eval-check.yml`, and its acceptance asks that a newcomer can trace a change using `DEVELOPMENT-FRAMEWORK.md` alone. Task PB-1.8 in `docs/TASKS.md` is this fix.

### Authority

- **Authoriser:** harry-nicholls. The merge is the approval.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `docs/assurance/DEVELOPMENT-FRAMEWORK.md` | "5. Deploy — the PR", `incident-eval-check.yml` bullet | bullet replaced |

### Test / Coverage Impact

- No code, test, invariant or eval changes. The bullet follows the existing behaviour that `node --test scripts/ci/incident-eval-check.test.mjs` covers (IE-2, IE-5).
- `docs/assurance/**` stays "not yet reached" in the tier gate's evidence table. No check reads the prose.

### Review Checklist

- [x] Rationale is anchored to the script, the workflow trigger, and the runs for #60 and #61.
- [x] Authoriser is a named human.
- [x] PB-1 covers `incident-eval-check.yml` and `DEVELOPMENT-FRAMEWORK.md`.
- [x] The diff plan names every changed protected file.
- [x] No check is weakened. Only prose changes.
- [ ] REQUIRES HUMAN VERIFICATION: The maintainer accepts that `docs/gates/tier-layer-gate.md` keeps its wrong claim until task PB-1.11, and that the surfaces PB-1.12 names (the check's failure message, `TIER-LAYER-MAP.md`, `evals/README.md`, and stage 6 of `DEVELOPMENT-FRAMEWORK.md`) keep theirs until that task.
