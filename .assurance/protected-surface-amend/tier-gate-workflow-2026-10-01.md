## Protected-Surface Amendment

**Target file(s):** `.github/workflows/tier-gate.yml`, `scripts/ci/tier-gate-workflow.test.mjs`
**Class:** A (CI enforcement)
**Matched rule:** `.github/workflows/**`, `scripts/ci/**`
**Date:** 2026-10-01

### Change Description

1. `.github/workflows/tier-gate.yml`: the `Compute changed files vs base` step is deleted. The `Run tier gate` step fetches the base, sets `CHANGED_FILES` from `git diff --name-only --no-renames "origin/$BASE_REF...HEAD"`, and runs the gate. `BASE_REF` comes from the step's `env:`, so no `run:` script contains a `${{ }}` expression. Nothing writes to `$GITHUB_OUTPUT`, so the fixed `EOF` heredoc delimiter is gone (TG-14).
2. `scripts/ci/tier-gate-workflow.test.mjs`: new. It runs the `Run tier gate` step's script from the workflow file in a scratch repository, with files named `EOF` and `a<<EOF` next to a protected file, with a renamed protected file, and with a base ref that names no branch (TG-15).

### Rationale

Task PB-1.7. A changed file named `EOF` ended the `$GITHUB_OUTPUT` block early. With a second file named `docs/assurance/a<<EOF`, the runner read the remaining file names as the value of another output, and the gate saw an empty list. Scratch pull request #60 changed the protected file `docs/assurance/zz.md`, declared `Tier: 1`, and the Tier Gate passed with `CHANGED_FILES` empty. The base ref was also substituted into shell source by a `${{ }}` expression. Intent: `intent/2026-10-01-tier-gate-workflow.md`. Spec: `intent/2026-10-01-tier-gate-workflow-spec.md`. Plan: `intent/2026-10-01-tier-gate-workflow-plan.md`.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (immediate horizon, item PB-1)
- **Title:** Adopt the AI-native SDLC playbook as the development framework and make every human gate self-explanatory
- **Scope coverage:** PB-1's scope names `tier-gate.yml` with the gate's own tests. Task PB-1.7 in `docs/TASKS.md` is this fix.

### Authority

- **Authoriser:** harry-nicholls. The merge is the approval.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `.github/workflows/tier-gate.yml` | `Compute changed files vs base`, `Run tier gate` | step deleted, step changed |
| 2 | `scripts/ci/tier-gate-workflow.test.mjs` | whole file | added |

### Test / Coverage Impact

- `node --test scripts/ci/*.test.mjs` gains the TG-15 cases. TG-1a (`--no-renames`) was untested and is now tested.
- The gate's result changes for a pull request with a file named `EOF`. With no `<name><<EOF` file, the job failed at the diff step and now runs the gate on the full list. With one, the gate saw only the files before `EOF` and now sees all of them.
- `scripts/ci/tier-gate.mjs`, its tests, and every invariant and eval are unchanged.

### Review Checklist

- [x] Rationale is anchored to a reproduction on GitHub (#60).
- [x] Authoriser is a named human.
- [x] PB-1 covers `tier-gate.yml` and its tests.
- [x] The diff plan names every changed protected file.
- [x] No check is weakened: the gate gets the same list, computed the same way, and only loses the `$GITHUB_OUTPUT` hop.
- [ ] The maintainer accepts that the test runs the step under local bash, not on a GitHub runner, and that the quoted-path fault waits for PB-1.10.
