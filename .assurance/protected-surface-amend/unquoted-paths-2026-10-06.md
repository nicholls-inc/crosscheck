## Protected-Surface Amendment

**Target file(s):** `.github/workflows/tier-gate.yml`, `scripts/ci/tier-gate.mjs`, `scripts/ci/tier-gate-workflow.test.mjs`
**Class:** A (CI enforcement)
**Matched rule:** `.github/workflows/**`, `scripts/ci/**`
**Date:** 2026-10-06

### Change Description

1. `.github/workflows/tier-gate.yml`: the `Run tier gate` step writes `git diff -z --name-only --no-renames "origin/$BASE_REF...HEAD"` to a `mktemp` file and exports its path as `CHANGED_FILES_PATH`, in place of the newline-separated `CHANGED_FILES` (TG-14 revised).
2. `scripts/ci/tier-gate.mjs`: the gate reads the changed files from `CHANGED_FILES_PATH`, splits on NUL, and keeps each name as written, with no trim. It no longer reads `CHANGED_FILES` (TG-16).
3. `scripts/ci/tier-gate-workflow.test.mjs`: three cases add `docs/assurance/é.md`, `docs/assurance/a"b.md` and `docs/assurance/n<newline>l.md` at `Tier: 1` and expect the Tier 3 floor (TG-15 revised).

### Rationale

Task PB-1.10. `git diff --name-only` C-quotes a name with a non-ASCII byte, a double quote, a backslash or a control character. The quoted name starts with `"` and matches no protected glob, so a change to `docs/assurance/é.md` passed the gate at `Tier: 1`. `core.quotePath=false` fixes only the non-ASCII case, and a newline-separated list cannot carry a name with a newline. Intent: `intent/2026-10-06-unquoted-paths.md`. Spec: `intent/2026-10-06-unquoted-paths-spec.md`. Plan: `intent/2026-10-06-unquoted-paths-plan.md`.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (immediate horizon, item PB-1)
- **Title:** Adopt the AI-native SDLC playbook as the development framework and make every human gate self-explanatory
- **Scope coverage:** PB-1's scope names `tier-gate.yml` with the gate's own tests. Task PB-1.10 in `docs/TASKS.md` is this fix.

### Authority

- **Authoriser:** harry-nicholls. The merge is the approval.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `.github/workflows/tier-gate.yml` | `Run tier gate` | `-z` list in a temporary file |
| 2 | `scripts/ci/tier-gate.mjs` | header comment, `main` | reads `CHANGED_FILES_PATH` |
| 3 | `scripts/ci/tier-gate-workflow.test.mjs` | new cases | added |

### Test / Coverage Impact

- `node --test scripts/ci/*.test.mjs` gains three TG-15 cases. They fail against the old workflow and gate.
- The gate's result changes only for a pull request with a name git quotes, which now counts against the protected globs, and for a name with leading or trailing spaces, which the gate no longer trims.
- No invariant or eval changes.

### Review Checklist

- [x] Rationale is anchored to a local reproduction of git's quoting.
- [x] Authoriser is a named human.
- [x] PB-1 covers `tier-gate.yml`, the gate and its tests.
- [x] The diff plan names every changed protected file.
- [x] No check is weakened: the gate sees every name it saw before, and the quoted ones as written.
- [ ] The maintainer accepts that `CHANGED_FILES` is gone, so a manual run must pass `CHANGED_FILES_PATH`.
