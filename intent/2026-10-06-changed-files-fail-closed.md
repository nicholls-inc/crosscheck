# Intent: The Tier Gate fails closed without its changed-file list

Task: PB-1.15. Governing roadmap item: PB-1.

## Problem statement
`readChangedFiles` in `scripts/ci/tier-gate.mjs` returns an empty list when `CHANGED_FILES_PATH` is unset or empty. With an empty list the gate sees no protected path, so a run whose body declares `Tier: 1` and cites an intent passes at Tier 1 whatever the branch changes. A local run in a scratch repository holding `scripts/ci/x.mjs` printed `tier-gate: PASS — Tier 1 artefacts present`. A manual run, or a workflow edit that drops or misspells the variable, therefore passes a protected change. When the variable names a file that does not exist, `readFileSync` throws and Node prints a stack trace with exit 1, which names neither the variable nor what to do.

## Proposed outcome
- With `CHANGED_FILES_PATH` unset or empty, the gate exits 1 and its failure names `CHANGED_FILES_PATH` and the command that writes the list.
- With `CHANGED_FILES_PATH` naming a file the gate cannot read (missing, a directory, unreadable), the gate exits 1 and its failure names `CHANGED_FILES_PATH`, the path and the error code, with no stack trace.
- Both failures carry the fixed gate message and explainer link, as TG-10 asks of every failure.
- An empty file still means an empty diff, so the gate's result for a readable file does not change.

## Affected users and systems
- Anyone who runs the gate by hand. They must now pass `CHANGED_FILES_PATH`.
- `scripts/ci/tier-gate.mjs` and `scripts/ci/tier-gate.test.mjs`. `.github/workflows/tier-gate.yml` already sets the variable and does not change.
- `docs/TASKS.md` and `JOURNAL.md`.

## Constraints
- No dependency is added.
- `evaluate` keeps its signature. `scripts/ci/pre-commit.mjs` imports from the gate and does not read `CHANGED_FILES_PATH`.
- The gate's other inputs do not change.

## Open questions
None. The task row states the outcome.
