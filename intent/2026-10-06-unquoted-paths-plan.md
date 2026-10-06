# Plan: The Tier Gate reads every changed file name as git stores it

Intent: `intent/2026-10-06-unquoted-paths.md`
Spec: `intent/2026-10-06-unquoted-paths-spec.md` (TG-16, TG-14 and TG-15 revised)
Governing roadmap item: PB-1. Task: PB-1.10. Tier: 3.

This plan is not the root `plan.md`, which belongs to an earlier change. The pull request body cites this file with a `Plan:` line.

## Order of work

1. Write the governance note `.assurance/protected-surface-amend/unquoted-paths-2026-10-06.md`, naming the three protected files below.
2. Add the three TG-15 cases to `scripts/ci/tier-gate-workflow.test.mjs`. Run `node --test scripts/ci/*.test.mjs` and see them fail against the current workflow: each file reaches the gate quoted and passes at Tier 1. Commit the test on its own.
3. In `scripts/ci/tier-gate.mjs`, read `CHANGED_FILES_PATH` as TG-16 says and stop reading `CHANGED_FILES`. In `.github/workflows/tier-gate.yml`, write the `-z` list to a `mktemp` file and export its path (TG-14).
4. Run `node --test scripts/ci/*.test.mjs` and `actionlint .github/workflows/tier-gate.yml`.
5. Update the input list in `intent/2026-09-29-deterministic-evidence-spec.md`.
6. Set PB-1.10 to `done` in `docs/TASKS.md`, with this intent as its record. Add row PB-1.13 for `.husky/commit-msg`. Add a root `JOURNAL.md` entry.

## Files

| File | Protected | Change |
|---|---|---|
| `.github/workflows/tier-gate.yml` | yes (`.github/workflows/**`) | `-z` list in a temporary file (TG-14) |
| `scripts/ci/tier-gate.mjs` | yes (`scripts/ci/**`) | reads `CHANGED_FILES_PATH` (TG-16) |
| `scripts/ci/tier-gate-workflow.test.mjs` | yes (`scripts/ci/**`) | three TG-15 cases |
| `intent/2026-09-29-deterministic-evidence-spec.md` | no | input list |
| `docs/TASKS.md` | no | PB-1.10 `done`, new row PB-1.13 |
| `JOURNAL.md` | no | entry |
| `.assurance/protected-surface-amend/unquoted-paths-2026-10-06.md` | no | new governance note |
| `intent/2026-10-06-unquoted-paths*.md` | no | stage artefacts |

## Risks

- **A name with leading or trailing spaces is no longer trimmed.** The gate now sees the real name. A trimmed name that existed nowhere on disk could never count as an artefact, so no pull request loses an artefact. A name such as ` docs/assurance/x.md`, which is not under `docs/assurance/`, no longer matches a protected glob, which is correct.
- **Anyone who ran the gate by hand with `CHANGED_FILES`** now gets an empty list and a pass on the floor. The workflow is the only caller in the repository.

## Proof that it worked

- `node --test scripts/ci/*.test.mjs` passes, and the three new cases fail against the old workflow and gate.
- This pull request's own Tier Gate run passes at Tier 3 and prints `base: main`.
