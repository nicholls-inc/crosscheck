# Intent: Print the next task, and check the task queue in CI

Task: PB-1.6. Governing roadmap item: PB-1.

## Problem statement
`docs/TASKS.md` is the ordered queue of tasks, and "Pick up the next task" in `docs/assurance/DEVELOPMENT-FRAMEWORK.md` says how an agent chooses and claims one. Nothing deterministic applies or checks any of it.

- An agent chooses the next task by reading the table by eye. The rules are mechanical (status `todo`, every dependency `done`, no `task/<task ID>` branch), but each agent applies them by hand, and two agents can read the same table two ways.
- Nothing checks the queue. A row can name a roadmap item that does not exist, depend on a task that does not exist, or carry a status outside `todo`, `blocked` and `done`. A reader is the only check.
- A `done` is self-attested. The pull request that completes a task sets its own row to `done`, and the pick-up procedure then treats that row as a met dependency. Nothing stops a pull request from setting a different row, or several rows, to `done`. The spec of the queue (`intent/2026-09-30-task-queue-spec.md`) flags both gaps and names this task as the fix.
- The claim step is a shell snippet in the framework document. Its safety against two agents claiming the same task at once rests on an argument in prose. No test runs it.

## Proposed outcome
- `node scripts/ci/task-queue.mjs next` prints the ID of the next task by the rules of step 1 of the pick-up procedure, read from `origin/main` and the remote's `task/*` branches. When no task is ready, it says so and lists each row that is not ready with the reason.
- `node scripts/ci/task-queue.mjs check` fails when a row's task ID names no roadmap item, when a dependency names no task in the queue, when a status is not one of the three values, or when the pull request sets to `done` any row other than the one its `Task:` line names.
- A new workflow runs the check on every pull request.
- A test runs the claim snippet from `DEVELOPMENT-FRAMEWORK.md` twice at once against a scratch remote, and shows that exactly one claim wins.
- The pick-up procedure names the script, and a gate explainer covers the new check.

## Affected users and systems
- Agents that pick up tasks. They run one command instead of reading the table.
- Everyone who opens a pull request. The new check reads the PR body's `Task:` line and the change to `docs/TASKS.md`.
- New: `scripts/ci/task-queue.mjs`, `scripts/ci/task-queue.test.mjs`, `.github/workflows/task-queue.yml`, `docs/gates/task-queue-check.md`. Changed: `docs/assurance/DEVELOPMENT-FRAMEWORK.md`, `docs/gates/README.md`, `docs/TASKS.md`, `JOURNAL.md`.
- The Tier Gate job, whose existing step `node --test scripts/ci/*.test.mjs` picks up the new tests.

## Constraints
- No CI job calls an LLM. The script has no dependencies, like the other scripts under `scripts/ci/`.
- The rules of the pick-up procedure and the format of `docs/TASKS.md` do not change. The script applies them; it does not redefine them.
- `docs/TASKS.md` stays outside the protected paths.
- The ruleset requires no status checks, so the new check informs the merge and cannot block it.
- No change to `tier-gate.yml`. Its workflow fixes are task PB-1.7.

## Open questions
None.
