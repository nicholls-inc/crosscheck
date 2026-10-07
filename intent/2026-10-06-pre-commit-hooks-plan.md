# Plan: Pre-commit hooks for the checks that need no PR body

Intent: `intent/2026-10-06-pre-commit-hooks.md`
Spec: `intent/2026-10-06-pre-commit-hooks-spec.md` (PC-1 to PC-8)
Governing roadmap item: PB-1. Task: PB-1.9. Tier: 3.

This plan is not the root `plan.md`, which belongs to an earlier change. The pull request body cites this file with a `Plan:` line.

## Order of work

1. Write the governance note `.assurance/protected-surface-amend/pre-commit-hooks-2026-10-06.md`, naming every protected file below.
2. Add `scripts/ci/pre-commit.test.mjs` with the PC-8 cases. Run it and see it fail, because `.husky/pre-commit` and `scripts/ci/pre-commit.mjs` do not exist.
3. In `scripts/ci/tier-gate.mjs`, export `loadProtectedGlobs`, `globToRegExp` and `isGovernanceNotePath`, and move the TG-5 comparison into an exported `unnamedProtectedFiles(protectedMatches, noteText)` that `evaluate` calls. In `scripts/ci/task-queue.mjs`, move QC-1 to QC-4 out of `checkQueue` into an exported `checkRows({ rows, itemIds })` that `checkQueue` calls. Run `node --test scripts/ci/*.test.mjs`: the existing tests pass unchanged.
4. Add `scripts/ci/pre-commit.mjs` and `.husky/pre-commit` (PC-1 to PC-7). Run the tests until PC-8 passes.
5. Add `.husky/pre-commit` to the Tier Gate row of `EVIDENCE_CLASSES` and of the evidence table in `docs/assurance/TIER-LAYER-MAP.md`. Add a tier gate test case for the new row.
6. Say in `docs/assurance/DEVELOPMENT-FRAMEWORK.md` (stage 5), `docs/gates/tier-layer-gate.md` and `docs/gates/task-queue-check.md` that the hook runs the rules that need no PR body, and how to install it.
7. Time the hook on this repository with a staged protected file and with a staged `docs/TASKS.md`.
8. Set PB-1.9 to `done` in `docs/TASKS.md` with this intent as its record. Add a root `JOURNAL.md` entry.

## Files

| File | Protected | Change |
|---|---|---|
| `scripts/ci/pre-commit.mjs` | yes (`scripts/ci/**`) | new, PC-2 to PC-7 |
| `scripts/ci/pre-commit.test.mjs` | yes (`scripts/ci/**`) | new, PC-8 |
| `scripts/ci/tier-gate.mjs` | yes (`scripts/ci/**`) | exports; TG-5 comparison moved into a function; `.husky/pre-commit` evidence row |
| `scripts/ci/tier-gate.test.mjs` | yes (`scripts/ci/**`) | evidence row case |
| `scripts/ci/task-queue.mjs` | yes (`scripts/ci/**`) | `checkRows` split out of `checkQueue` |
| `docs/assurance/DEVELOPMENT-FRAMEWORK.md` | yes (`docs/assurance/**`) | stage 5 names the hook |
| `docs/assurance/TIER-LAYER-MAP.md` | yes (`docs/assurance/**`) | evidence table row |
| `.husky/pre-commit` | no | new, PC-1 |
| `docs/gates/tier-layer-gate.md`, `docs/gates/task-queue-check.md` | no | the hook and its install |
| `docs/TASKS.md`, `JOURNAL.md` | no | PB-1.9 `done`, entry |
| `.assurance/protected-surface-amend/pre-commit-hooks-2026-10-06.md` | no | new governance note |
| `intent/2026-10-06-pre-commit-hooks*.md` | no | stage artefacts |

## Risks

- **The refactor changes CI.** Step 3 moves code that CI runs. The existing tier gate and task queue tests pin its behaviour, and they must pass unchanged before step 4.
- **The hook blocks commits it should not.** PC-6 keeps it to commits that stage a protected path or a queue file. A clone with no `origin` blocks only a commit that stages a protected path, and the message says `git fetch origin`.
- **Old git.** `--merge-base` needs git 2.30. An older git fails the `git diff` call, and the hook exits 2 with git's message.

## Proof that it worked

- `node --test scripts/ci/*.test.mjs` passes, including PC-8, and PC-8 fails before step 4.
- On this repository, with the hook installed through `core.hooksPath`, a commit that stages a protected file with no note fails with the `Fix:` line, and the time is under 5 seconds.
- This pull request's own Tier Gate and Task Queue runs pass.
