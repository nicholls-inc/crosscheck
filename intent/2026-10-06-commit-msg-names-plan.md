# Plan: The commit-msg hook reads every staged file name as git stores it

Intent: `intent/2026-10-06-commit-msg-names.md`
Spec: `intent/2026-10-06-commit-msg-names-spec.md` (CM-1 to CM-5)
Governing roadmap item: PB-1. Task: PB-1.13. Tier: 3.

The hook itself is not protected. The test is, because `scripts/ci/**` is, and CI runs only `scripts/ci/*.test.mjs`. `scripts/ci/protected-surface-guard.test.mjs` already tests a hook from there.

## Order of work

1. Write the governance note `.assurance/protected-surface-amend/commit-msg-names-2026-10-06.md`, naming `scripts/ci/commit-msg.test.mjs`.
2. Add `scripts/ci/commit-msg.test.mjs` with the CM-5 cases. Run `node --test scripts/ci/*.test.mjs` and see the four unusual-name `docs:` cases and the `refactor:` case fail against the current hook. Commit the test on its own.
3. In `.husky/commit-msg`, read the names as CM-3 says.
4. Run `node --test scripts/ci/*.test.mjs`. Check each new case by mutation: revert the hook, drop `*agents/*.md` from the pattern, make the check never block, and make it block every `docs:` commit. Each mutation fails at least one case.
5. Set PB-1.13 to `done` in `docs/TASKS.md`, with the intent as its record. Add a root `JOURNAL.md` entry.

## Files

| File | Protected | Change |
|---|---|---|
| `scripts/ci/commit-msg.test.mjs` | yes (`scripts/ci/**`) | new, CM-5 |
| `.husky/commit-msg` | no | CM-3 |
| `docs/TASKS.md` | no | PB-1.13 `done` |
| `JOURNAL.md` | no | entry |
| `.assurance/protected-surface-amend/commit-msg-names-2026-10-06.md` | no | new governance note |
| `intent/2026-10-06-commit-msg-names*.md` | no | stage artefacts |

## Risks

- **`xargs` on empty input.** GNU `xargs` runs the command once with no arguments, and BSD `xargs` does not run it. The loop does nothing with no arguments, so both give an empty list. Checked under dash with GNU `xargs` (`python:3.11-slim`) and under macOS `sh` with BSD `xargs`.
- **A long list.** `xargs` may split the names over several `sh` runs. Each run prints its matches, and the output is their concatenation, so no name is lost.

## Proof that it worked

- `node --test scripts/ci/*.test.mjs` passes, and the new cases fail against the old hook.
- The scratch repository repro from the intent prints `exit=1` for all four names.
