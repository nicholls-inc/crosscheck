# Plan: The tier gate accepts any citation line, and only a regular file in the repository

Intent: `intent/2026-09-30-citation-rule.md`
Spec: `intent/2026-09-30-citation-rule-spec.md` (TG-12, TG-13, DOC-7)
Governing roadmap item: PB-1. Task: PB-1.5. Tier: 3.

This plan is not the root `plan.md`, which belongs to an earlier change. The pull request body cites this file with a `Plan:` line.

## Order of work

1. Write the governance note `.assurance/protected-surface-amend/citation-rule-2026-09-30.md`, naming the three protected files below.
2. Add the TG-13 cases to `scripts/ci/tier-gate.test.mjs`. Run `node --test scripts/ci/*.test.mjs` and see the new positive cases for several lines and the new negative cases for directories, outside paths and symlinks fail against the current gate. Commit the tests on their own.
3. In `scripts/ci/tier-gate.mjs`, change `citedExisting`:
   - read every match of the citation pattern with `matchAll` and the `gim` flags, and return true if any cited path is valid;
   - a path is valid when `realpathSync(resolve(cwd, path))` succeeds, `statSync` of it is a regular file, and its path relative to `realpathSync(cwd)` neither starts with `..` nor is absolute.
4. Run `node --test scripts/ci/*.test.mjs`.
5. Update the documents in DOC-7.
6. Set PB-1.5 to `done` in `docs/TASKS.md`, with this intent as its record, and add a root `JOURNAL.md` entry.

## Files

| File | Protected | Change |
|---|---|---|
| `scripts/ci/tier-gate.mjs` | yes (`scripts/ci/**`) | `citedExisting` (TG-12) |
| `scripts/ci/tier-gate.test.mjs` | yes (`scripts/ci/**`) | TG-13 cases |
| `docs/assurance/TIER-LAYER-MAP.md` | yes (`docs/assurance/**`) | citation wording (DOC-7) |
| `docs/gates/tier-layer-gate.md` | no | citation wording (DOC-7) |
| `intent/2026-09-29-deterministic-evidence-spec.md` | no | pointer from TG-2 to this spec |
| `docs/TASKS.md` | no | PB-1.5 `done` |
| `JOURNAL.md` | no | entry |
| `.assurance/protected-surface-amend/citation-rule-2026-09-30.md` | no | new governance note |
| `intent/2026-09-30-citation-rule*.md` | no | stage artefacts |

## Risks

- **A PR that cited a directory or a path outside the repository.** It now fails. The fix is to cite the file itself. The PR bodies of #52 to #57 cite files under `intent/`.
- **A body with several citation lines.** It can now pass where it failed. Each counting line still has to name a regular file in the repository, so no requirement is weakened.
- **Symlinks.** `realpathSync` follows every link, so a link inside the repository to a file inside it passes, and a link to a file outside fails.

## Proof that it worked

- `node --test scripts/ci/*.test.mjs` passes, including every TG-13 case, and the new cases that assert the fixed behaviour fail against the old gate.
- The repro that drove this task, five bodies run against a scratch repository, gives the wanted result for each.
- The gate, run locally with this branch's changed files and the PR body, passes at Tier 3.
