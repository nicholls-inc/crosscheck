# Plan: The protected-surface hook matches a path that holds a newline

Intent: `intent/2026-10-06-hook-newline.md`
Spec: `intent/2026-10-06-hook-newline-spec.md` (PG-9)
Governing roadmap item: PB-1. Task: PB-1.14. Tier: 3.

This plan is not the root `plan.md`, which belongs to an earlier change. The pull request body cites this file with a `Plan:` line.

## Order of work

1. Write the governance note `.assurance/protected-surface-amend/hook-newline-2026-10-06.md`, naming the two protected files below.
2. Add the three PG-9 cases to `scripts/ci/protected-surface-guard.test.mjs`. Run `node --test scripts/ci/*.test.mjs` and see all three fail against the current hook, which exits 0 where each expects 2. Commit the test on its own.
3. In `.claude/hooks/protected-surface-guard.mjs`, compile each glob with the `s` flag.
4. Run `node --test scripts/ci/*.test.mjs`.
5. Point PG-6 in `intent/2026-09-30-merged-notes-unlock-spec.md` at PG-9.
6. Set PB-1.14 to `done` in `docs/TASKS.md`, with this intent as its record, and add row PB-1.17 for the pre-commit hook on a merge commit. Add a root `JOURNAL.md` entry.

## Files

| File | Protected | Change |
|---|---|---|
| `.claude/hooks/protected-surface-guard.mjs` | yes (`.claude/hooks/**`) | `s` flag in `globToRegExp` (PG-9) |
| `scripts/ci/protected-surface-guard.test.mjs` | yes (`scripts/ci/**`) | three PG-9 cases |
| `intent/2026-09-30-merged-notes-unlock-spec.md` | no | PG-6 points at PG-9 |
| `docs/TASKS.md` | no | PB-1.14 `done`, new row PB-1.17 |
| `JOURNAL.md` | no | entry |
| `.assurance/protected-surface-amend/hook-newline-2026-10-06.md` | no | new governance note |
| `intent/2026-10-06-hook-newline*.md` | no | stage artefacts |

## Risks

- **The hook blocks more edits.** Only edits to paths that hold a newline under a `**` glob change, from allowed to blocked. No such path exists in the repository.

## Proof that it worked

- `node --test scripts/ci/*.test.mjs` passes, and the three PG-9 cases fail against the old hook.
- The local repro from the intent prints `exit=2` for both newline paths.
