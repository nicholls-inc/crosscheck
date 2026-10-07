## Protected-Surface Amendment

**Target file(s):** `scripts/ci/pre-commit.mjs`, `scripts/ci/pre-commit.test.mjs`, `scripts/ci/tier-gate.mjs`, `scripts/ci/tier-gate.test.mjs`, `scripts/ci/task-queue.mjs`, `docs/assurance/DEVELOPMENT-FRAMEWORK.md`, `docs/assurance/TIER-LAYER-MAP.md`
**Class:** A (CI enforcement, governance documents)
**Matched rule:** `scripts/ci/**`, `docs/assurance/**`
**Date:** 2026-10-06

### Change Description

1. `scripts/ci/pre-commit.mjs` (new): run by `.husky/pre-commit`. It reads the commit from the index. When the commit stages a protected path, it fails unless every protected path changed on the branch is named in a governance note changed on the branch (TG-5, PC-3). When the commit stages `docs/TASKS.md` or `docs/assurance/ROADMAP.md`, it fails on QC-1 to QC-4 (PC-4). Each failure prints a `Fix:` command and the rerun command (PC-5). It never fetches, and it reads nothing beyond the staged list and the rules file for an unrelated commit (PC-6).
2. `scripts/ci/pre-commit.test.mjs` (new): the PC-8 cases, each a real `git commit` through the hook in a scratch clone, with a 5-second budget on each run of the hook process, timed from git's trace2 events.
3. `scripts/ci/tier-gate.mjs`: exports `loadProtectedGlobs`, `globToRegExp` and `isGovernanceNotePath`. The TG-5 comparison moves into an exported `unnamedProtectedFiles`, which `evaluate` calls. `.husky/pre-commit` joins the Tier Gate row of `EVIDENCE_CLASSES`. The gate's result on any pull request does not change, apart from that report row.
4. `scripts/ci/tier-gate.test.mjs`: a case for the new evidence row.
5. `scripts/ci/task-queue.mjs`: QC-1 to QC-4 move out of `checkQueue` into an exported `checkRows`, which `checkQueue` calls. The check's result does not change.
6. `docs/assurance/DEVELOPMENT-FRAMEWORK.md`: stage 5 names the hook and the rules it runs.
7. `docs/assurance/TIER-LAYER-MAP.md`: the evidence table maps `.husky/pre-commit` to the Tier Gate.

### Rationale

Task PB-1.9 in `docs/TASKS.md`. The roadmap's dual-track enforcement principle (`docs/assurance/ROADMAP.md`) asks every deterministic check for a pre-commit hook as well as a CI job, and the tier gate and the task queue check have only the CI job. Intent: `intent/2026-10-06-pre-commit-hooks.md`. Spec: `intent/2026-10-06-pre-commit-hooks-spec.md`. Plan: `intent/2026-10-06-pre-commit-hooks-plan.md`.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (immediate horizon, item PB-1)
- **Title:** Adopt the AI-native SDLC playbook as the development framework and make every human gate self-explanatory
- **Scope coverage:** PB-1 covers the deterministic checks of the framework, the tier gate and the task queue among them, and the roadmap's dual-track principle applies to each. Task PB-1.9 is this change.

### Authority

- **Authoriser:** harry-nicholls. The merge is the approval.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `scripts/ci/pre-commit.mjs` | whole file | added |
| 2 | `scripts/ci/pre-commit.test.mjs` | whole file | added |
| 3 | `scripts/ci/tier-gate.mjs` | exports, TG-5 block, `EVIDENCE_CLASSES` | replaced |
| 4 | `scripts/ci/tier-gate.test.mjs` | evidence report cases | added |
| 5 | `scripts/ci/task-queue.mjs` | `checkQueue` | replaced |
| 6 | `docs/assurance/DEVELOPMENT-FRAMEWORK.md` | "5. Deploy" | reworded |
| 7 | `docs/assurance/TIER-LAYER-MAP.md` | evidence table | reworded |

### Test / Coverage Impact

- `node --test scripts/ci/*.test.mjs`, run by the Tier Gate job, gains the PC-8 cases and one evidence row case. The existing tier gate and task queue tests pass unchanged, which pins the refactor.
- No gate rule, tier, protected path, invariant or eval changes. The hook checks a subset of the CI rules and adds none.

### Review Checklist

- [x] Rationale is anchored to task PB-1.9 and the dual-track principle.
- [x] Authoriser is a named human.
- [x] PB-1 covers the tier gate, the task queue check and the dual-track principle.
- [x] The CI rules are unchanged; the existing tests pass without edits.
- [x] The hook never fetches, calls no LLM, and an unrelated commit needs no `origin`.
