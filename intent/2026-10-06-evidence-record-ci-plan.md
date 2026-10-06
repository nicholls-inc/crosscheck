# Plan: CI runs the evidence record checker's tests

Intent: `intent/2026-10-06-evidence-record-ci.md`
Spec: `intent/2026-10-06-evidence-record-ci-spec.md` (EC-1 to EC-3, TG-8 row 5a)
Governing roadmap item: ER-1. Task: ER-1.5. Tier: 3.

This plan is not the root `plan.md`, which belongs to an earlier change. The pull request body cites this file with a `Plan:` line.

## Order of work

1. Write the governance note `.assurance/protected-surface-amend/evidence-record-ci-2026-10-06.md`, naming the five protected files below.
2. Add `scripts/ci/evidence-record-workflow.test.mjs` with the EC-3 cases, and the EC-2 cases in `scripts/ci/tier-gate.test.mjs`. Run `node --test scripts/ci/*.test.mjs` and see them fail: `evidence-record.yml` does not exist, and the gate reports the checker as row 10.
3. Add `.github/workflows/evidence-record.yml` as EC-1 writes it.
4. Add row 5a to `EVIDENCE_CLASSES` in `scripts/ci/tier-gate.mjs`.
5. Run `node --test scripts/ci/*.test.mjs`, `node --test scripts/check-evidence-record.test.mjs` and `actionlint .github/workflows/evidence-record.yml`.
6. Add row 5a to the evidence table of `docs/assurance/TIER-LAYER-MAP.md`. Point TG-8 in `intent/2026-09-30-tier-anchor-spec.md` at EC-2.
7. Set ER-1.5 to `done` in `docs/TASKS.md`, with the intent as its record. Add a root `JOURNAL.md` entry.

## Files

| File | Protected | Change |
|---|---|---|
| `.github/workflows/evidence-record.yml` | yes (`.github/workflows/**`) | new, EC-1 |
| `scripts/ci/evidence-record-workflow.test.mjs` | yes (`scripts/ci/**`) | new, EC-3 |
| `scripts/ci/tier-gate.mjs` | yes (`scripts/ci/**`) | row 5a in `EVIDENCE_CLASSES` |
| `scripts/ci/tier-gate.test.mjs` | yes (`scripts/ci/**`) | EC-2 cases |
| `docs/assurance/TIER-LAYER-MAP.md` | yes (`docs/assurance/**`) | row 5a in the evidence table |
| `intent/2026-09-30-tier-anchor-spec.md` | no | TG-8 points at EC-2 |
| `docs/TASKS.md` | no | ER-1.5 `done` |
| `JOURNAL.md` | no | entry |
| `.assurance/protected-surface-amend/evidence-record-ci-2026-10-06.md` | no | new governance note |
| `intent/2026-10-06-evidence-record-ci*.md` | no | stage artefacts |

## Risks

- **A second check on every pull request.** The checker's tests take about 4 seconds. Without a path filter the check runs on every pull request, so its result never depends on which files changed.
- **The EC-3 test spawns `node --test` inside `node --test`.** The parent sets `NODE_TEST_CONTEXT` for its children, and a child `node --test` that inherits it reports to the parent instead of printing TAP. The test removes the variable from the child's environment.
- **The test reads the YAML by indentation.** If the step is renamed or its `run:` moves, the test fails with a message that names the step, not silently.

## Proof that it worked

- `node --test scripts/ci/*.test.mjs` passes, and the EC-2 and EC-3 cases fail before steps 3 and 4.
- Replacing the step's `run:` with `true`, or pointing it at a missing file, fails the first EC-3 case.
- This pull request's own Evidence Record run passes and its log shows the 125 checker tests.
