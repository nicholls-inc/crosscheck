# Spec: CI runs the evidence record checker's tests

Intent: `intent/2026-10-06-evidence-record-ci.md`. Governing roadmap item: ER-1. Task: ER-1.5.

This spec adds EC-1 to EC-3 and revises TG-8 of `intent/2026-09-30-tier-anchor-spec.md` by one row. The checker, its tests and the other TG requirements do not change.

- **EC-1. The workflow.** `.github/workflows/evidence-record.yml` is named `Evidence Record`.
  - It runs on `pull_request` events of type `opened`, `synchronize` and `reopened`, with no path filter, and with `contents: read` as its only permission.
  - One job, `evidence-record`, on `ubuntu-latest`, checks out the repository with `actions/checkout@v6`, sets up Node 24 with `actions/setup-node@v6`, and has a step named `Test the evidence record checker` whose `run:` is `node --test scripts/check-evidence-record.test.mjs`.
  - No `run:` script in the workflow contains `${{`.
  - A failing checker test makes the step, and so the check, fail.
- **EC-2. Evidence report.** TG-8's table gains a row between row 5 (`scripts/ci/**`) and row 6 (`evals/**`):

  | # | Changed path | Kind | Workflow |
  |---|---|---|---|
  | 5a | `scripts/check-evidence-record.mjs`, `scripts/check-evidence-record.test.mjs` | checked | `Evidence Record workflow (node --test scripts/check-evidence-record.test.mjs)` |

  The row matches those two paths exactly. Any other path under `scripts/` still falls to row 10. `docs/assurance/TIER-LAYER-MAP.md`'s evidence table shows the same row.
- **EC-3. Tests.**
  - `scripts/ci/evidence-record-workflow.test.mjs` reads the `run:` script of the step named `Test the evidence record checker` from `evidence-record.yml`, by indentation, and runs it with `bash -e` from a scratch directory that holds copies of the checker and its tests under `scripts/`. It covers:
    - with the checker as committed, the script exits 0, and its output, in the TAP format that the test selects through `NODE_OPTIONS`, reports at least one passing test and no failing test (EC-1);
    - with a checker whose `checkRecord` returns an empty list, the script exits non-zero, and its TAP summary reports at least one failing and at least one passing test, so a crash or a file that fails to load does not satisfy it (EC-1);
    - the workflow file, read with comments, trailing whitespace, blank lines and CRLF line endings ignored, has no `if:` key, no `continue-on-error`, no `paths`, `paths-ignore`, `branches`, `branches-ignore`, `tags` or `tags-ignore` filter and no `${{`; its `on:` block holds `pull_request` with the `types` of EC-1 and nothing else; and it has exactly one `permissions:` key, `contents: read` (EC-1).

    The first case fails if the step runs another command or a path that does not exist. The second fails if the step does not run the checker's tests, or ignores their result.
  - `scripts/ci/tier-gate.test.mjs` gains a case: a pass whose changed files are the two paths in EC-2 and `scripts/other.mjs` reports the two under the Evidence Record workflow and `scripts/other.mjs` as row 10 (EC-2). The "every class" case gains `scripts/check-evidence-record.mjs`.
- **Known gaps, not rules.**
  - The EC-3 test runs the step's script on the test machine, not on a GitHub runner, so the `on:` trigger and the job setup are not in it. This pull request's own Evidence Record run is the runner evidence. A workflow is "not yet reached" in TG-8 for this reason.
  - The row in EC-2 names two files by path. A new file that the checker imports would fall to row 10 until the row names it.
