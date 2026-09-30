# Plan: A task queue that any agent can pick up from

Intent: `intent/2026-09-30-task-queue.md`
Spec: `intent/2026-09-30-task-queue-spec.md` (requirement IDs RM-*, TQ-*, PU-*, PT-*)
Governing roadmap item: PB-1. Tier: 3. Task: PB-1.1.

This file replaces the root `plan.md` from #46, which remains in git history.

## Order of work

1. **Governance note first.** Write `.assurance/protected-surface-amend/task-queue-2026-09-30.md`. It names `docs/assurance/ROADMAP.md` and `docs/assurance/DEVELOPMENT-FRAMEWORK.md`. The tier gate counts a note only if the pull request changes it (TG-5).
2. **Roadmap (RM-1 to RM-6).** In `docs/assurance/ROADMAP.md`:
   - replace the strategic context with one that cites `docs/VISION.md`;
   - replace the three `TODO` rows with nine items, each with a status, a scope, an acceptance statement, and its issues;
   - extend the scope of PB-1;
   - add the subsection "Tasks".
3. **Queue (TQ-1 to TQ-7).** Add `docs/TASKS.md` with the column definitions, the rules, and the first rows.
4. **Procedure (PU-1 to PU-8).** Add the section "Pick up the next task" to `docs/assurance/DEVELOPMENT-FRAMEWORK.md`, before "Which agent runs which stretch".
5. **Pointers (PT-1, PT-2).** Add a short section to `CLAUDE.md` and to `AGENTS.md`, and one line to the layout in `README.md`.
6. **Journal.** Add an entry to the root `JOURNAL.md` that says why the queue and the roadmap are separate files.

## Files

| File | Protected | Change |
|---|---|---|
| `docs/assurance/ROADMAP.md` | yes (`docs/assurance/**`) | RM-1 to RM-6 |
| `docs/assurance/DEVELOPMENT-FRAMEWORK.md` | yes (`docs/assurance/**`) | PU-1 to PU-8 |
| `.assurance/protected-surface-amend/task-queue-2026-09-30.md` | no | new governance note |
| `docs/TASKS.md` | no | new; TQ-1 to TQ-7 |
| `CLAUDE.md`, `AGENTS.md` | no | PT-1 |
| `README.md` | no | PT-2 |
| `JOURNAL.md` | no | one entry |
| `intent/2026-09-30-task-queue*.md`, `plan.md` | no | stage artefacts |

No `SKILL.md`, agent, hook, rule, CI, or code file changes.

## Risks

- **An agent wrote the roadmap items.** The maintainer's merge approves nine items that the maintainer did not write. Mitigation: the governance note marks the items `REQUIRES HUMAN VERIFICATION`, and the review checklist lists them.
- **The queue drifts from the truth.** A pull request can merge without marking its row. Mitigation: the procedure puts the row change in the same pull request, and the `Task:` line in the body lets a reviewer check it. No deterministic check exists until task PB-1.6.
- **Two sources for status.** The roadmap has a status for each item and the queue has one for each task. Mitigation: only the maintainer changes the status of an item (PU-8), and the queue never states the status of an item.
- **A stale claim.** A `task/<task ID>` branch that nobody finishes blocks its task. The procedure reports claimed tasks instead of taking them over.
- **The procedure is prose.** An agent applies PU-1 by reading. A different agent could read it differently. Mitigation: the three conditions are mechanical, and PB-1.6 turns them into a script.

## Proof that it worked

- The tier gate passes when run locally against this branch's changed files and the pull request body, and prints "not yet reached: human review is the only evidence" for `docs/assurance/**`.
- `node --test scripts/ci/*.test.mjs` still passes.
- A script outside the repository reads `docs/TASKS.md` and `docs/assurance/ROADMAP.md` and confirms each of these:
  - every task ID names a roadmap item that exists, and no ID repeats (TQ-2);
  - every status is `todo`, `blocked`, or `done` (TQ-3);
  - every dependency is a task in the queue (TQ-4);
  - no horizon table holds `TODO` (RM-1);
  - the issues that the roadmap items name are exactly the open issues from `gh issue list` (RM-4);
  - the first row that meets PU-1 is `PB-1.2`.
- Every relative link in the changed Markdown files resolves to a file.
- The claim commands in step 2 were run by hand against a scratch bare remote with two and three clones. One agent won in the same-base race, in the sequential case, and after `main` moved. The loser's push was rejected with `stale info`, and its clean-up left no local `task/*` branch. The nonce guard stopped when `uuidgen` printed nothing. A claim with no commit of its own printed "Everything up-to-date" and exited 0, which is why the commit is needed. This is a manual run, not a committed check: task `PB-1.6` adds a test that races two claims.
- Not covered: whether an agent that starts with no context and is told "pick up next task" reaches `PB-1.2` and follows the chain. Only a fresh session can show that.
