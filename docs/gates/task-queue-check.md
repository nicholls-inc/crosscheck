# Gate: Task Queue Check (CI Failure)

## What this gate protects

`docs/TASKS.md` is the ordered queue of work on this repository. An agent told to "pick up next task" takes the first row whose status is `todo`, whose every dependency is `done`, and that no `task/<task ID>` branch claims (see "Pick up the next task" in `docs/assurance/DEVELOPMENT-FRAMEWORK.md`). So a wrong row changes what the next agent works on. A row marked `done` too early lets the tasks that depend on it start before their groundwork has merged.

The **task-queue CI job** (`scripts/ci/task-queue.mjs check`, run by `.github/workflows/task-queue.yml`) runs on every pull request. It reads the queue in the pull request and the queue on the base branch, and fails when:

- a task ID is not of the form `<roadmap item ID>.<n>`, or names an item that `docs/assurance/ROADMAP.md` does not define;
- two rows have the same task ID;
- a status is not `todo`, `blocked` or `done`;
- a dependency names a task that is not in the queue;
- the pull request sets a row to `done` that its `Task:` line does not name.

A **roadmap item** is a numbered piece of work in `docs/assurance/ROADMAP.md`, such as PB-1, that governs a set of tasks. The `Task:` line is a line in the PR body, such as `Task: PB-1.6`. It must start the line, optionally after an indent, with no list or quote marker before it, and the first such line counts. A pull request with no `Task:` line may not set any row to `done`.

## When the check cannot read a queue

The check exits 2, and prints the file and the problem instead of the gate message, when:

- the queue table, in the pull request or on the base branch, has no separator row (`|---|---|...|`) under its header. Without one, the first task row cannot be told from a separator, and skipping it could hide a row set to `done`;
- `docs/TASKS.md` exists on the base branch but has no `## Queue` table with the expected header. Only a missing file means an empty base queue, so the check does not count every `done` row as newly done.

Fix the separator row in the pull request. If the base branch's queue is the broken one, the pull request that repairs it cannot pass the check, because there is no base to compare with. The maintainer reads that pull request's diff and merges it, and the check reads the repaired queue from then on.

## What each decision means

- **Approving (fixing the queue or the `Task:` line)**: the queue on the default branch stays one that the pick-up procedure can read, and only the pull request that does a task marks it done.
- **Declining (leaving the queue as it is)**: the check stays red. The ruleset on the default branch requires no status checks, so GitHub does not stop the merge. The maintainer does not merge while this check is red.

## What the check does not catch

It does not check that the pull request sets its own task to `done`, that a task ID was never used before, that dependencies form no cycle, or that a `blocked` row says what unblocks it. These are listed under "Concerns flagged" in `intent/2026-10-01-queue-check-spec.md`.

## How long this takes

Usually a minute: fix the row the message names, or add the `Task:` line. Run `BASE_REF=main PR_BODY="$(gh pr view --json body -q .body)" node scripts/ci/task-queue.mjs check` locally to see the same result.

## Who to ask if unsure

The Crosscheck maintainers, via a GitHub issue on this repository.
