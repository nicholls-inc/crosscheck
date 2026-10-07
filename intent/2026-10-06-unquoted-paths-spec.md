# Spec: The Tier Gate reads every changed file name as git stores it

Intent: `intent/2026-10-06-unquoted-paths.md`. Governing roadmap item: PB-1. Task: PB-1.10.

This spec adds TG-16 and revises TG-14 and TG-15 of `intent/2026-10-01-tier-gate-workflow-spec.md` and the input list of `intent/2026-09-29-deterministic-evidence-spec.md`. The other TG requirements do not change.

- **TG-16. NUL-separated names.** `scripts/ci/tier-gate.mjs` reads the changed files from the file that `CHANGED_FILES_PATH` names. The file holds names separated by NUL bytes, as `git diff -z --name-only` writes them. The gate drops empty entries and keeps every other entry byte for byte, with no trim and no unquoting. With `CHANGED_FILES_PATH` unset, the list was empty, as an unset `CHANGED_FILES` was before. That failed open, and this task kept it. **Revised by TG-17 of `intent/2026-10-06-changed-files-fail-closed-spec.md` (PB-1.15): the gate now fails closed when the variable is unset, empty or names a file it cannot read.** `CHANGED_FILES` is no longer read. A protected glob's `**` matches any characters, newlines included, so `docs/assurance/**` matches `docs/assurance/n<newline>l.md`.
- **TG-14, revised. The workflow.** The `Run tier gate` script runs `git fetch origin "$BASE_REF" --depth=1`, sets `CHANGED_FILES_PATH` to the output of `mktemp`, writes `git diff -z --name-only --no-renames "origin/$BASE_REF...HEAD"` to that file, exports `CHANGED_FILES_PATH`, and runs `node scripts/ci/tier-gate.mjs`. Each is its own command, so a failed `mktemp` or `git diff` fails the step under `bash -e`. The step does not remove the file, since the runner is discarded after the job. The other TG-14 rules (`BASE_REF` from `env:`, no `${{` in a `run:` script, nothing written to `$GITHUB_OUTPUT`) stand.
- **TG-15, revised. Tests.** `scripts/ci/tier-gate-workflow.test.mjs` adds three cases. A branch that adds one of these files at `Tier: 1` fails, and the output names the Tier 3 floor and the file as written:
  - `docs/assurance/é.md`;
  - `docs/assurance/a"b.md`;
  - `docs/assurance/n<newline>l.md`.
- **Known gaps, not rules.**
  - The gate decodes the file as UTF-8. A name that is not valid UTF-8 keeps its valid prefix, so a protected directory prefix still matches, but the gate cannot find that file on disk and will not count it as an artefact.
  - The test runs the script under local bash, not on a GitHub runner. This pull request's own Tier Gate run is the runner evidence.
