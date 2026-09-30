# Spec: A task queue that any agent can pick up from

Intent: `intent/2026-09-30-task-queue.md`. Governing roadmap item: PB-1.

Each requirement has an ID. The plan and the pull request cite these IDs.

## Roadmap (`docs/assurance/ROADMAP.md`)

- **RM-1.** Every horizon table lists at least one item. No table holds a `TODO` row.
- **RM-2.** Each new item has a stable ID, a `Status:` of `Not started`, a scope, an acceptance statement, and the open issues that belong to it.
- **RM-3.** Each of the seven design rules in `docs/VISION.md`, the second link of its chain (the spec and the assumptions give the requirement), and its scope section maps to at least one item. The item names the rule.
- **RM-4.** Every open GitHub issue on 2026-09-30 belongs to exactly one item.
- **RM-5.** PB-1 and MR-1 keep their IDs and their status. The scope of PB-1 gains the task queue and the pick-up procedure.
- **RM-6.** The roadmap states that `docs/TASKS.md` holds the tasks, and that only the maintainer changes the status of an item.

## Queue (`docs/TASKS.md`)

- **TQ-1.** The queue is one Markdown table. The order of the rows is the order of work.
- **TQ-2.** A task ID is `<roadmap item ID>.<n>`, for example `PB-1.2`. The roadmap item must exist. An ID is never reused.
- **TQ-3.** A status is one of `todo`, `blocked`, or `done`. A `blocked` row states what unblocks it. The queue has no `in progress` status: a pushed branch named `task/<task ID>` is the claim (PU-2).
- **TQ-4.** A row lists the tasks it depends on, its GitHub issue if it has one, and its record. The record of a `done` task is the path of its intent file.
- **TQ-5.** One task is one pull request.
- **TQ-6.** The file is not a protected surface. It grants no authority: a protected-surface change cites a roadmap item, never a task.
- **TQ-7.** The first queue holds tasks only for items in the Immediate and Next horizons, plus one task for each later item that already has open issues.

## Pick-up procedure (`docs/assurance/DEVELOPMENT-FRAMEWORK.md`)

- **PU-1. Choose.** The next task is the first row, read from `origin/main`, that meets all three conditions: its status is `todo`, every task it depends on is `done`, and no branch claims it.
- **PU-2. Claim.** The agent pushes a branch named exactly `task/<task ID>` that holds one empty commit with a unique message on top of `origin/main`, with a lease that fails if the branch already exists (`--force-with-lease=refs/heads/task/<task ID>:`). A failed push, or a remote SHA that is not the agent's own commit, means that another agent holds the task. A plain push is not enough: a second push of a branch with no commit of its own names the same SHA, and git reports success.
- **PU-3. Read.** Before it writes the intent, the agent reads `docs/VISION.md`, the governing roadmap item, the linked issue, and the journals that `AGENTS.md` names.
- **PU-4. Run the chain.** The agent follows the stages of `DEVELOPMENT-FRAMEWORK.md` from stage 1, at the tier that `TIER-LAYER-MAP.md` gives.
- **PU-5. Record.** The pull request body has a `Task: <task ID>` line. The same pull request sets the row to `done` and fills in the record.
- **PU-6. Stop for a person.** The agent stops and reports in each of these cases:
  - the intent has an open question;
  - no row meets PU-1;
  - the work needs a protected-surface change that no roadmap item covers.
- **PU-7. New work.** An agent that finds new work adds a `todo` row under an existing roadmap item, or opens an issue. It does not start that work in the current pull request.
- **PU-8. Item status.** An agent does not change the `Status:` of a roadmap item. When the last task of an item is done, the pull request body says so. The maintainer decides whether the item meets its acceptance.

## Pointers

- **PT-1.** `CLAUDE.md` and `AGENTS.md` each tell an agent that "pick up next task" means the procedure in `DEVELOPMENT-FRAMEWORK.md`. Neither file repeats the procedure.
- **PT-2.** `README.md` lists `docs/TASKS.md` in the repository layout.

## Concerns flagged, not resolved here

- **The proposed items are a draft.** An agent wrote the scope, acceptance, horizon, and order of the nine new items from `docs/VISION.md` and the open issues. None of that is the maintainer's decision until the merge. Several acceptance statements name no number, because no baseline exists yet.
- **The queue is not checked.** Nothing deterministic checks TQ-1 to TQ-4. A row with a bad ID or a missing dependency is caught only by a reader. Task `PB-1.6` in the first queue adds the check.
- **A claim needs the remote.** PU-2 works only for an agent that can push. An agent without push access can read the queue but cannot claim a task.
- **A stale claim blocks a task.** A `task/<task ID>` branch that nobody finishes keeps its task claimed until someone deletes the branch. The procedure tells the agent to report a claimed task, not to take it over.
- **The queue is outside the protected paths.** Any pull request can reorder it at Tier 1. The maintainer's merge is the only review. Protecting the file would make every status change a Tier 3 change with a governance note.
- **Merged governance notes unlock the hook.** `.claude/hooks/protected-surface-guard.mjs` accepts any note under `.assurance/protected-surface-amend/` that mentions a path. The three notes on `main` mention 29 of the 57 tracked files that match a protected glob, so the hook allows edits to those 29 today. Two of them are `cgv/prover/ContractGraph/BehaviorModel.lean` and `crosscheck/skills/reason/SKILL.md`, which one note mentions only as test examples. The tier gate is not affected (TG-5). This change does not fix the hook. Task `PB-1.3` does.
