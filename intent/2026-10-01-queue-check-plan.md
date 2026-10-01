# Plan: Print the next task, and check the task queue in CI

Intent: `intent/2026-10-01-queue-check.md`
Spec: `intent/2026-10-01-queue-check-spec.md`
Governing roadmap item: PB-1. Task: PB-1.6. Tier: 3.

This plan is not the root `plan.md`, which belongs to an earlier change. The pull request body cites this file with a `Plan:` line.

## Data shape

One parsed row type, `{ id, status, what, dependsOn: string[], issue, record }`, read from the table by one parser (QP-1, QP-2). Both subcommands are pure functions over rows:

- `nextTask(rows, claimedIds)` returns either `{ id }` or `{ reasons: [{ id, reason }] }` (NX-1, NX-2).
- `checkQueue({ rows, baseRows, itemIds, prBody })` returns a list of problem strings, empty on pass (QC-1 to QC-5).

The shell (`main`) reads git and the environment, calls one function, prints, and sets the exit code. Only the shell touches git.

## Order of work

1. Write the governance note `.assurance/protected-surface-amend/queue-check-2026-10-01.md`, naming the four protected files below.
2. Write `scripts/ci/task-queue.test.mjs` with TT-1 to TT-4. Run `node --test scripts/ci/task-queue.test.mjs` and see it fail, since the script does not exist.
3. Write `scripts/ci/task-queue.mjs` to the spec. Run `node --test scripts/ci/*.test.mjs` until it passes.
4. Write `.github/workflows/task-queue.yml` (WF-1).
5. Update `DEVELOPMENT-FRAMEWORK.md` (DOC-1, DOC-2), add `docs/gates/task-queue-check.md` and its inventory row (DOC-3).
6. Set PB-1.6 to `done` in `docs/TASKS.md`, add row PB-1.9, and add a root `JOURNAL.md` entry.
7. Run `check` locally with `BASE_REF=main` and this PR's body, and run `next` against the real remote.

## Files

| File | Protected | Change |
|---|---|---|
| `scripts/ci/task-queue.mjs` | yes (`scripts/ci/**`) | new: `next` and `check` |
| `scripts/ci/task-queue.test.mjs` | yes (`scripts/ci/**`) | new: TT-1 to TT-4 |
| `.github/workflows/task-queue.yml` | yes (`.github/workflows/**`) | new: WF-1 |
| `docs/assurance/DEVELOPMENT-FRAMEWORK.md` | yes (`docs/assurance/**`) | DOC-1, DOC-2 |
| `docs/gates/task-queue-check.md` | no | new explainer |
| `docs/gates/README.md` | no | inventory row 17 |
| `docs/TASKS.md` | no | PB-1.6 `done`, new row PB-1.9 |
| `JOURNAL.md` | no | entry |
| `.assurance/protected-surface-amend/queue-check-2026-10-01.md` | no | new governance note |
| `intent/2026-10-01-queue-check*.md` | no | stage artefacts |

## Risks

- **The table format drifts.** A column renamed or reordered makes `check` fail on QP-1. That is the intended result: the check names the header it expects.
- **The race test depends on `bash` and `uuidgen`.** Both exist on `ubuntu-latest` and on macOS. If `uuidgen` is missing, the snippet exits 1 by design, both runs fail, and TT-3 fails loudly rather than passing.
- **The race test reads prose.** An edit to the claim snippet in `DEVELOPMENT-FRAMEWORK.md` changes what the test runs. That is the point: the documented procedure is what agents run. If the block cannot be found, the test fails.
- **Concurrent runs may not overlap.** Two spawned processes can run one after the other. TT-3 then shows the same thing as TT-4. Five rounds make an overlap likely but not certain. Either way the result must be exactly one winner.
- **A PR body without a `Task:` line that marks a row `done`.** The check fails where nothing failed before. The fix is to add the `Task:` line.

## Proof that it worked

- `node --test scripts/ci/*.test.mjs` passes, including TT-1 to TT-4.
- `check` with `BASE_REF=main` and this PR's body passes. With the `Task:` line removed, it fails on PB-1.6.
- `next` against the real remote prints a task, and the claimed `task/*` branches are skipped.
