# Task queue

This file is the ordered queue of work on this repository. The order of the rows is the order of work. To take a task, follow "Pick up the next task" in [`assurance/DEVELOPMENT-FRAMEWORK.md`](assurance/DEVELOPMENT-FRAMEWORK.md).

## Columns

- **Task.** `<roadmap item ID>.<n>`. The item is in [`assurance/ROADMAP.md`](assurance/ROADMAP.md) and governs the task. An ID is never reused.
- **Status.** One of three values:
  - `todo`: nobody has finished the task.
  - `blocked`: the task waits for something outside the queue, such as a decision by a person or a fix elsewhere. The row says what unblocks it.
  - `done`: the pull request that completed the task is merged.

  The queue has no "in progress" status. A pushed branch named `task/<task ID>` shows that an agent holds the task.
- **Depends on.** Tasks that must be `done` first.
- **Issue.** The GitHub issue where the discussion is, if one exists.
- **Record.** The path of the intent file, once the task has one.

## Rules

- One task is one pull request. If a task is too large for one pull request, its pull request replaces the row with smaller rows, points every `Depends on` that named the old row at the new rows, and does nothing else.
- The pull request that completes a task sets its row to `done` and fills in the record.
- Add a row under an existing roadmap item. If no item fits, the work needs a new roadmap item first, and only the maintainer approves one.
- The queue grants no authority. A change to a protected surface cites a roadmap item.

## Queue

| Task | Status | What | Depends on | Issue | Record |
|---|---|---|---|---|---|
| PB-1.1 | done | Add this queue, the roadmap items for the vision, and the pick-up procedure | | | `intent/2026-09-30-task-queue.md` |
| PB-1.2 | done | Fix the `Incident Eval Check` workflow. It failed on the four squash-merged pull requests #43 to #46 with "Invalid revision range" | | | `intent/2026-09-30-incident-eval-range.md` |
| PB-1.3 | done | Stop merged governance notes from unlocking the protected-surface hook. The notes on `main` allow edits to 29 of 57 protected files | | | `intent/2026-09-30-merged-notes-unlock.md` |
| PB-1.4 | done | Tier gate: anchor the `Tier:` line, and label unchecked code "not yet reached" | | #50 | `intent/2026-09-30-tier-anchor.md` |
| PB-1.5 | todo | Tier gate: accept any citation line, require a regular file in the repository, and add negative tests | | #49 | |
| PB-1.6 | todo | Add a script that prints the next task by the rules of the pick-up procedure, and check this file in CI: each task ID names a roadmap item, each dependency exists, each status is valid, and a pull request sets to `done` only the row in its `Task:` line. Add a test that races two claims against a scratch remote | | | |
| PB-1.7 | todo | Tier Gate workflow: pass the base ref to `run:` as an environment variable, not a `${{ }}` expression, and stop writing the changed files through a fixed `EOF` heredoc delimiter, which a file named `EOF` ends early | | | |
| PB-1.8 | todo | `docs/assurance/DEVELOPMENT-FRAMEWORK.md` says `incident-eval-check.yml` fails on an incident record without an eval. State its real trigger (the `incident` label or an incident line in the body or a commit), its exit 2, and that it runs after the merge and cannot block it | | | |
| VA-1.1 | todo | Skills and agents stop presenting the `intent-check` attestation as a required artefact | | | |
| VA-1.2 | todo | Replace "out of scope", "not addressed" and "best-effort" in `crosscheck/README.md` and `crosscheck/docs/assurance-hierarchy.md` with "not yet reached", the blocking property, and the open question | | | |
| ER-1.1 | todo | Write the intent and the spec for the evidence record format | | | |
| ER-1.2 | todo | CGV emits an evidence record | ER-1.1 | | |
| ER-1.3 | todo | One Crosscheck pipeline emits an evidence record | ER-1.1 | | |
| ER-1.4 | todo | Add a deterministic checker for evidence records | ER-1.1 | | |
| CG-1.1 | todo | CGV README: say what the tool is for relative to type checkers, and qualify the exit 0 claim | | #10 | |
| CG-1.2 | todo | Reduce false-positive errors: numeric tower, object parameters, `NoReturn`, narrowing, pydantic validation | | #5 | |
| CG-1.3 | todo | Close the extractor gaps that turn real nullability bugs into warnings | | #6 | |
| CG-1.4 | todo | Hide missing-guarantee warnings by default, and report them as coverage for each module | | #9 | |
| CG-1.5 | todo | Baseline mode: report only the findings that a change introduces | | #7 | |
| CG-1.6 | todo | Give each error a checkable witness: a concrete value or a generated failing test | | #8 | |
| TB-1.1 | todo | CGV CI: replay the Lean kernel, so that a declaration that skipped the kernel cannot pass the axiom check | | #47 | |
| TB-1.2 | todo | CGV: reject `implemented_by` and `extern` on constants that the soundness theorems reach | | #48 | |
| TB-1.3 | todo | CGV manifest: hash the definitions that protected statements mention, and check the name lists against the rules table | | #51 | |
| TB-1.4 | todo | Write the intent and the plan for proving extraction, and split the work into rows | | #16 | |
| AD-1.1 | todo | Review issues #19 to #41 against `docs/VISION.md` and record one decision for each. Refine: rewrite the issue against the vision, name the rule or item it serves, and add a row under that item. Drop: close the issue with the reason. Start no work on any of them in this task | | #27 | |
