# Spec: Print the next task, and check the task queue in CI

Intent: `intent/2026-10-01-queue-check.md`. Governing roadmap item: PB-1. Task: PB-1.6.

Each requirement has an ID. The plan, the tests and the pull request cite these IDs.

## Reading the files

- **QP-1. Queue table.** The queue is the first Markdown table after the `## Queue` heading of `docs/TASKS.md`, whose header row is `| Task | Status | What | Depends on | Issue | Record |`. Cells split on `|` that is not escaped as `\|`. Each cell is trimmed. If the heading or the header row is missing, the file has no queue, and `check` fails.
- **QP-2. Dependencies.** The `Depends on` cell is a comma-separated list of task IDs. Backticks around an ID are ignored. An empty cell means no dependencies.
- **QP-3. Roadmap items.** A roadmap item exists when `docs/assurance/ROADMAP.md` has a line that starts `**<item ID> — Status:`. Every item has one (RM-2 in `intent/2026-09-30-task-queue-spec.md`).

## `next`

- **NX-1. Choose.** `node scripts/ci/task-queue.mjs next` reads the queue from `git show origin/main:docs/TASKS.md` and the claims from `git ls-remote --heads origin 'task/*'`. It prints the ID of the first row, in table order, whose status is `todo`, whose every dependency is a row with status `done`, and for which no branch `refs/heads/task/<task ID>` exists. It prints the ID alone on one line and exits 0.
- **NX-2. No task ready.** If no row meets NX-1, it prints `no task ready`, then one line for each row that is not `done` and not chosen, in table order, with its reason: `claimed by task/<task ID>`, `waiting on <IDs that are not done>`, or `blocked: <What cell>`. It exits 1.
- **NX-3. Read failure.** If a git command fails, it prints the command and its error and exits 2.
- **NX-4. No fetch.** The script does not fetch. Step 1 of the procedure runs `git fetch origin` first.

## `check`

`node scripts/ci/task-queue.mjs check` reads `docs/TASKS.md` and `docs/assurance/ROADMAP.md` from the working tree, the base queue from `git show origin/$BASE_REF:docs/TASKS.md`, and the PR body from `PR_BODY`. It fails, exit 1, with one line for each problem. It exits 2 when `BASE_REF` is unset or the git command fails for a reason other than a missing file.

- **QC-1. Task ID.** Each task ID has the form `<item ID>.<n>`, where the item ID is uppercase letters, a hyphen and digits, and `n` is a positive integer. The item exists in the roadmap (QP-3).
- **QC-2. Unique.** No two rows have the same task ID.
- **QC-3. Status.** Each status is `todo`, `blocked` or `done`.
- **QC-4. Dependencies exist.** Each dependency names a row of the queue.
- **QC-5. Done only for the task.** A row is newly done when its status is `done` and the base queue has no row with that ID whose status is `done`. If a row is newly done, its ID equals the PR body's task. The PR body's task is the trimmed rest of the first line that matches `^[ \t]*Task:(.*)$`, case-insensitive, with no list or quote marker before the keyword, the same anchor as the tier gate's `Tier:` line (TG-1 in `intent/2026-09-30-tier-anchor-spec.md`). A body with no `Task:` line sets no row to `done`. If the base branch has no `docs/TASKS.md`, every `done` row is newly done.
- **QC-6. Gate message.** On failure, the output starts with the four-line gate message of `docs/gates/README.md`, and its `Full explanation:` link names `docs/gates/task-queue-check.md`. On pass, it prints one line with the number of rows checked and the number newly done.

## Workflow

- **WF-1.** `.github/workflows/task-queue.yml` runs on `pull_request` (`opened`, `edited`, `synchronize`, `reopened`). It checks out with `fetch-depth: 0`, sets up Node 24, and runs `node scripts/ci/task-queue.mjs check`. `BASE_REF` and `PR_BODY` reach the script as environment variables. No `run:` line contains a `${{ }}` expression. Permissions are `contents: read`.

## Tests (`scripts/ci/task-queue.test.mjs`)

The Tier Gate job runs them with `node --test scripts/ci/*.test.mjs`. Each test names the requirement it covers.

- **TT-1.** `check` passes on the queue at `origin/main`, and on each failure in QC-1 to QC-5 it fails and names the row.
- **TT-2.** `next` against a scratch remote: it skips a claimed row, a row with a dependency that is not `done`, and a `blocked` row, and it prints the first ready row. With no row ready, it exits 1 and prints a reason for each row.
- **TT-3. Claim race.** The test reads the claim snippet, the first fenced `bash` block after `2. **Claim.**` in `docs/assurance/DEVELOPMENT-FRAMEWORK.md`, and replaces `<task ID>` with a test ID. It runs the snippet at the same time in two clones of one scratch bare remote, under `bash -e`, with the same git identity. Exactly one run exits 0. The remote branch's SHA equals that run's `HEAD`. The other run exits non-zero, and its `HEAD` differs from the remote SHA. The test repeats the race five times, each on a fresh remote.
- **TT-4. Claim after claim.** A second run of the snippet, after the first has pushed, exits non-zero and leaves the remote branch at the first run's SHA.

## Documents

- **DOC-1.** Step 1 of "Pick up the next task" in `DEVELOPMENT-FRAMEWORK.md` names `node scripts/ci/task-queue.mjs next` as the command that applies its three conditions. The conditions stay in the prose.
- **DOC-2.** Stage 5 of `DEVELOPMENT-FRAMEWORK.md` lists `task-queue.yml` with what it fails on.
- **DOC-3.** `docs/gates/task-queue-check.md` explains the check, and `docs/gates/README.md` lists it in the inventory.
- **DOC-4.** `docs/TASKS.md` sets PB-1.6 to `done` with this intent as its record.

## Concerns flagged, not resolved here

- **The completing PR need not set its own row to `done`.** QC-5 checks that a PR sets to `done` only its own task. It does not check that it sets its own task. PU-5 asks for both, and a pull request that replaces a row with smaller rows (the queue's split rule) sets nothing to `done`. Requiring the row to be set would fail that split PR.
- **An ID can be reused.** The queue says an ID is never reused. Checking that needs every ID ever in the history of the file, not only the base. A row deleted in one PR and re-added in a later one passes QC-1 to QC-5.
- **A dependency cycle passes the check.** QC-4 checks that each dependency exists, not that the graph is acyclic. A cycle shows up in `next` as rows waiting on each other.
- **A `done` row can return to `todo`.** QC-5 checks only transitions into `done`.
- **No pre-commit hook.** The roadmap's dual-track principle asks for a pre-commit hook and a CI job for each check. The tier gate has none either. `node scripts/ci/task-queue.mjs check` runs locally, but nothing runs it on commit. A new row, PB-1.9, covers pre-commit hooks for both checks.
- **The race test runs locally against a bare repository.** It shows that the lease and the unique commit decide the race on git's own ref update. It does not exercise GitHub's server. The assumption is that GitHub's receive path applies the same compare-and-swap on the ref.
