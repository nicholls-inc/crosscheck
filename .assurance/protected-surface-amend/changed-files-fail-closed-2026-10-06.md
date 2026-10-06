## Protected-Surface Amendment

**Target file(s):** `scripts/ci/tier-gate.mjs`, `scripts/ci/tier-gate.test.mjs`
**Class:** A (CI enforcement)
**Matched rule:** `scripts/ci/**`
**Date:** 2026-10-06

### Change Description

1. `scripts/ci/tier-gate.mjs`: when `CHANGED_FILES_PATH` is unset or empty, or names a file the gate cannot read, the gate prints the TG-10 failure lines with an item naming `CHANGED_FILES_PATH` and exits 1. It no longer reads an unset variable as an empty list, and no longer prints a stack trace for a missing file (TG-17).
2. `scripts/ci/tier-gate.test.mjs`: seven cases run the gate as a script with the variable unset, empty, naming a missing file, naming a directory, and naming three readable lists (TG-18).

### Rationale

Task PB-1.15. An unset `CHANGED_FILES_PATH` gave the gate an empty list, so it saw no protected path and passed a `Tier: 1` declaration whatever the branch changed. A manual run, or a workflow edit that dropped or misspelled the variable, passed a protected change. A missing file crashed with a stack trace. Intent: `intent/2026-10-06-changed-files-fail-closed.md`. Spec: `intent/2026-10-06-changed-files-fail-closed-spec.md`. Plan: `intent/2026-10-06-changed-files-fail-closed-plan.md`.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (immediate horizon, item PB-1)
- **Title:** Adopt the AI-native SDLC playbook as the development framework and make every human gate self-explanatory
- **Scope coverage:** PB-1's scope names `tier-gate.yml` with the gate's own tests. Task PB-1.15 in `docs/TASKS.md` is this fix.

### Authority

- **Authoriser:** harry-nicholls. The merge is the approval.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `scripts/ci/tier-gate.mjs` | header comment, `readChangedFiles`, `main` | fails closed on `CHANGED_FILES_PATH` |
| 2 | `scripts/ci/tier-gate.test.mjs` | new cases | added |

### Test / Coverage Impact

- `node --test scripts/ci/*.test.mjs` gains seven TG-18 cases. The three failure cases fail against the old gate.
- The gate's result changes only for a run without a readable `CHANGED_FILES_PATH`, which now fails. A run with a readable list, as the workflow makes, gets the same result as before.
- No invariant or eval changes.

### Review Checklist

- [x] Rationale is anchored to the gate's code path for an unset variable.
- [x] Authoriser is a named human.
- [x] PB-1 covers the gate and its tests.
- [x] The diff plan names every changed protected file.
- [x] No check is weakened: every run that passed with a readable list still passes, and runs without one now fail.
- [ ] REQUIRES HUMAN VERIFICATION: the maintainer accepts that a manual run without `CHANGED_FILES_PATH` now fails.
