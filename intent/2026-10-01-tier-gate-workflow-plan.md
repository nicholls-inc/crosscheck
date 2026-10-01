# Plan: The Tier Gate workflow reads the base ref from the environment and every changed file name

Intent: `intent/2026-10-01-tier-gate-workflow.md`
Spec: `intent/2026-10-01-tier-gate-workflow-spec.md` (TG-14, TG-15, TG-1a revised)
Governing roadmap item: PB-1. Task: PB-1.7. Tier: 3.

This plan is not the root `plan.md`, which belongs to an earlier change. The pull request body cites this file with a `Plan:` line.

## Order of work

1. Write the governance note `.assurance/protected-surface-amend/tier-gate-workflow-2026-10-01.md`, naming the two protected files below.
2. Add `scripts/ci/tier-gate-workflow.test.mjs` with the TG-15 cases. Run `node --test scripts/ci/*.test.mjs` and see the cases fail against the current workflow: its `Run tier gate` script is only `node scripts/ci/tier-gate.mjs`, so `CHANGED_FILES` is unset and every branch passes at Tier 1. Commit the test on its own.
3. In `.github/workflows/tier-gate.yml`, delete the `Compute changed files vs base` step. Give `Run tier gate` a `BASE_REF` entry in `env:` and the script in TG-14.
4. Run `node --test scripts/ci/*.test.mjs` and `actionlint .github/workflows/tier-gate.yml`.
5. Revise TG-1a in `intent/2026-09-29-deterministic-evidence-spec.md` to point at TG-15.
6. Set PB-1.7 to `done` in `docs/TASKS.md`, with this intent as its record. Add row PB-1.10 for the quoted-path fault. Add a root `JOURNAL.md` entry.

## Files

| File | Protected | Change |
|---|---|---|
| `.github/workflows/tier-gate.yml` | yes (`.github/workflows/**`) | one step computes the files and runs the gate (TG-14) |
| `scripts/ci/tier-gate-workflow.test.mjs` | yes (`scripts/ci/**`) | new, TG-15 cases |
| `intent/2026-09-29-deterministic-evidence-spec.md` | no | TG-1a points at TG-15 |
| `docs/TASKS.md` | no | PB-1.7 `done`, new row PB-1.10 |
| `JOURNAL.md` | no | entry |
| `.assurance/protected-surface-amend/tier-gate-workflow-2026-10-01.md` | no | new governance note |
| `intent/2026-10-01-tier-gate-workflow*.md` | no | stage artefacts |

## Risks

- **The gate step now fails when `git diff` fails.** Before, a failed diff in the first step also failed the job, so no pull request changes result for that reason.
- **The base fetch is unchanged.** `--depth=1` on top of a full checkout still leaves the merge base reachable, because `actions/checkout` with `fetch-depth: 0` fetched the whole history first.
- **The test reads the YAML by indentation.** If the step is renamed or its `run:` stops being a `|` block, the test fails with a message that names the step, not silently.

## Proof that it worked

- `node --test scripts/ci/*.test.mjs` passes, including every TG-15 case, and the TG-15 cases fail against the old workflow.
- Scratch pull request #60 runs the fixed workflow after its branch takes the change. The Tier Gate then fails on the Tier 3 floor and names `docs/assurance/zz.md`, where it passed at Tier 1 before.
- This pull request's own Tier Gate run passes at Tier 3 and prints `base: main`.
