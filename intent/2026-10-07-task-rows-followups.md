# Intent: Queue the follow-ups that the 2026-10-07 reviews found

Governing roadmap items: VA-1, PB-1, TB-1. This pull request adds rows to `docs/TASKS.md` and does no task.

## Problem statement
The reviews of the 15 task pull requests that landed on 2026-10-07 found six follow-ups that no row tracks. Without a row, the next "pick up next task" does not see them, and the finding lives only in a closed review thread.

## Proposed outcome
`docs/TASKS.md` gains six `todo` rows, each under an existing roadmap item:
- VA-1.16: two docs still say a layer checks "the spec is the right spec".
- VA-1.17: `/rationale` wording that #77 did not reach, and two small slips in the VA-1.5 plan and note.
- VA-1.18: the `ROADMAP.md` line on pre-commit hooks, and the VA-1.1 journal entry's `Touches` list.
- PB-1.52: the protected-surface hook guards `Edit`, `Write` and `NotebookEdit`, so a write through Bash skips it.
- TB-1.37: the flaky `interrupting_the_cli_kills_the_checker` test.
- TB-1.38: a helper name that the plan and note get wrong, and a "not yet reached" sentence that points at a spec instead of stating its open question.

## Affected users and systems
`docs/TASKS.md` only. No protected surface changes, so the tier gate's floor does not apply.

## Constraints
- Each row sits under an existing roadmap item. No item is invented.
- Rows use "not yet reached" wording, with the blocking property and the open question, and never "excluded" or "out of scope" (`docs/VISION.md`).
- Rows 2, 3 and 6 name edits to protected files. Their pull requests will need a governance note. This pull request edits none of those files.

## Open questions
Which roadmap item fits the flaky test: TB-1 (the trusted base includes the checker binary's process handling) or CG-1. This pull request files it under TB-1; the maintainer may move it.
