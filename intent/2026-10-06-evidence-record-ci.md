# Intent: CI runs the evidence record checker's tests

Task: ER-1.5. Governing roadmap item: ER-1.

## Problem statement
`scripts/check-evidence-record.mjs` (ER-1.4) is the deterministic checker that ER-1's acceptance asks for. Its 125 tests in `scripts/check-evidence-record.test.mjs` run only when someone runs them by hand. The Tier Gate workflow runs `node --test scripts/ci/*.test.mjs`, and the checker sits in `scripts/`, so no workflow runs its tests. A pull request that breaks the checker merges with every check green.

The tier gate's pass report says the same thing. A change to either file falls to the last row of its evidence table and reads "not yet reached: no CI workflow runs a check on this path".

## Proposed outcome
- A new workflow, `.github/workflows/evidence-record.yml` (Evidence Record), runs `node --test scripts/check-evidence-record.test.mjs` on every pull request. A failing checker test fails the check.
- The tier gate's evidence table gains a row: `scripts/check-evidence-record.mjs` and `scripts/check-evidence-record.test.mjs` are checked by the Evidence Record workflow. `docs/assurance/TIER-LAYER-MAP.md` shows the same row.
- A test under `scripts/ci/` runs the workflow step's own `run:` script. It passes against the real checker and fails against a checker whose `checkRecord` reports no problems. The Tier Gate job runs it with the other `scripts/ci/*.test.mjs` tests.
- `docs/TASKS.md` marks ER-1.5 `done` with this file as its record.

## Affected users and systems
- Whoever changes the checker: CI now tells them when a checker test fails.
- Whoever reads an evidence record: a checker that CI tests is one they can rely on more.
- `.github/workflows/evidence-record.yml` (new), `scripts/ci/tier-gate.mjs`, `scripts/ci/tier-gate.test.mjs`, `scripts/ci/evidence-record-workflow.test.mjs` (new), `docs/assurance/TIER-LAYER-MAP.md`. All five are Class A protected surfaces, so the change is Tier 3.
- `intent/2026-09-30-tier-anchor-spec.md` (TG-8), `docs/TASKS.md` and `JOURNAL.md`.

## Constraints
- No new dependency. The workflow uses the same `actions/checkout@v6` and `actions/setup-node@v6` with Node 24 as the Tier Gate.
- The Tier Gate's result does not change. Its tests still run first, and its evidence report stays information only.
- The checker and its tests do not change.
- The test reads the YAML without a YAML parser, as `scripts/ci/tier-gate-workflow.test.mjs` does.
- The ruleset requires no status checks, so the new check informs the merge and cannot block it.

## Open questions
None. The task row states the outcome. One choice is made here rather than left open. The tests run in their own workflow, not as a step of the Tier Gate job, so a checker failure reads as "Evidence Record" failing rather than as the Tier Gate failing. The spec records the choice.
