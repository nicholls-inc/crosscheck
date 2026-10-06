# Plan: The Tier Gate fails closed without its changed-file list

Intent: `intent/2026-10-06-changed-files-fail-closed.md`
Spec: `intent/2026-10-06-changed-files-fail-closed-spec.md` (TG-17, TG-18, TG-16 revised)
Governing roadmap item: PB-1. Task: PB-1.15. Tier: 3.

This plan is not the root `plan.md`, which belongs to an earlier change. The pull request body cites this file with a `Plan:` line.

## Order of work

1. Write the governance note `.assurance/protected-surface-amend/changed-files-fail-closed-2026-10-06.md`, naming the two protected files below.
2. Add the four TG-18 cases to `scripts/ci/tier-gate.test.mjs`. Run `node --test scripts/ci/*.test.mjs` and see the three failure cases fail against the current gate: unset and empty pass at Tier 1, and the missing file prints a stack trace. Commit the tests on their own.
3. In `scripts/ci/tier-gate.mjs`, make `readChangedFiles` return either the list or a failure item, and make `main` print the TG-10 failure lines and exit 1 on a failure item (TG-17). Update the header comment.
4. Run `node --test scripts/ci/*.test.mjs`. Mutate the fix back (return `[]` for an unset variable, rethrow on a read error) and see the cases fail.
5. Set PB-1.15 to `done` in `docs/TASKS.md`, with this intent as its record. Add a root `JOURNAL.md` entry.

## Files

| File | Protected | Change |
|---|---|---|
| `scripts/ci/tier-gate.mjs` | yes (`scripts/ci/**`) | fails closed on `CHANGED_FILES_PATH` (TG-17) |
| `scripts/ci/tier-gate.test.mjs` | yes (`scripts/ci/**`) | four TG-18 cases |
| `docs/TASKS.md` | no | PB-1.15 `done` |
| `JOURNAL.md` | no | entry |
| `.assurance/protected-surface-amend/changed-files-fail-closed-2026-10-06.md` | no | new governance note |
| `intent/2026-10-06-changed-files-fail-closed*.md` | no | stage artefacts |

## Risks

- **A manual run without `CHANGED_FILES_PATH` now fails.** That is the point of the task. The failure says how to write the list.
- **The workflow step.** `.github/workflows/tier-gate.yml` exports `CHANGED_FILES_PATH` before it runs the gate, so this pull request's own Tier Gate run shows the workflow still sets it.

## Proof that it worked

- `node --test scripts/ci/*.test.mjs` passes, and the three failure cases fail against the old gate.
- This pull request's own Tier Gate run passes at Tier 3 and prints `base: main`.
