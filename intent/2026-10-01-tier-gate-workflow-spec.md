# Spec: The Tier Gate workflow reads the base ref from the environment and every changed file name

Intent: `intent/2026-10-01-tier-gate-workflow.md`. Governing roadmap item: PB-1. Task: PB-1.7.

This spec adds TG-14 and TG-15, and revises TG-1a of `intent/2026-09-29-deterministic-evidence-spec.md`. The other TG requirements do not change, and `scripts/ci/tier-gate.mjs` does not change.

- **TG-14. The workflow.** In `.github/workflows/tier-gate.yml`, the step that runs the gate also computes its changed files.
  - The step's `env:` sets `BASE_REF` to `${{ github.event.pull_request.base.ref }}`, with `PR_BODY` and `PR_LABELS` as before. No `run:` script in the workflow contains `${{`.
  - The script runs `git fetch origin "$BASE_REF" --depth=1`, sets `CHANGED_FILES` to the output of `git diff --name-only --no-renames "origin/$BASE_REF...HEAD"`, exports it, and runs `node scripts/ci/tier-gate.mjs`.
  - The assignment and the `export` are separate commands, so a failed `git diff` fails the step under `bash -e`. `export CHANGED_FILES="$(...)"` would hide the failure.
  - Nothing in the workflow writes to `$GITHUB_OUTPUT`, so a file name cannot end or open an output block.
- **TG-1a, revised. Renames.** The changed-file list is computed with `--no-renames`, so a protected file moved to another path counts as a change to the protected path. `tier-gate.yml` holds this, and TG-15 tests it.
- **TG-15. Tests.** `scripts/ci/tier-gate-workflow.test.mjs` reads the `run:` script of the step named `Run tier gate` from `tier-gate.yml` and runs it with `bash -e` in a clone of a scratch remote. The clone holds `scripts/ci/tier-gate.mjs`, `.claude/rules/protected-surfaces.md` and an intent file from this repository. Each case sets `BASE_REF=main` and a `PR_BODY` of `Tier: 1` with an `Intent:` citation, and covers:
  - a branch that adds `EOF`, `docs/assurance/a<<EOF` and `docs/assurance/zz.md` fails, and the output names the Tier 3 floor and `docs/assurance/zz.md` (TG-14);
  - a branch that adds `EOF` and `notes.md` passes at Tier 1, and the pass line names `base: main` (TG-14);
  - a branch that renames `docs/assurance/x.md` to `notes/x.md` fails on the Tier 3 floor (TG-1a);
  - a `BASE_REF` that names no branch fails the step, and the gate does not run (TG-14).

  A `${{ }}` expression left in the script fails the first three cases, because bash rejects `${{` as a bad substitution and the gate never runs.
- **Known gaps, not rules.**
  - The test runs the script under `bash -e` on the test machine, not on a GitHub runner. The runner's own handling of `env:` and of `$GITHUB_OUTPUT` is not in the test. Scratch pull request #60 is the runner evidence for the fault, and this pull request's own Tier Gate run is the runner evidence for the fix.
  - `git diff --name-only` quotes a path with a byte outside printable ASCII, so such a path matches no protected glob. Task PB-1.10 fixes it.
