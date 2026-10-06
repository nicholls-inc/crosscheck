# Plan: Describe the Incident Eval Check the same way everywhere

Intent: `intent/2026-10-06-incident-eval-surfaces.md`
Spec: `intent/2026-10-06-incident-eval-surfaces-spec.md` (IE-2, IE-6, IE-8, TG-8 row 6, DOC-7)
Governing roadmap item: PB-1. Task: PB-1.12. Tier: 3.

This plan is not the root `plan.md`, which belongs to an earlier change. The pull request body cites this file with a `Plan:` line.

## Order of work

1. Write the governance note `.assurance/protected-surface-amend/incident-eval-surfaces-2026-10-06.md`, naming the seven protected files below.
2. Code and tests, in one commit:
   - `scripts/ci/incident-eval-check.mjs`: `GATE_DOC` becomes `incident-eval-check.md`; `printFailure` prints the IE-8 text; the empty-range comment names a merge commit and drops the rebase merge (IE-2).
   - `scripts/ci/incident-eval-check.test.mjs`: `failureOutput` expects the IE-8 text and link; a new helper builds a rebase-merged pull request, and two new tests cover it (IE-6).
   - `scripts/ci/tier-gate.mjs`: row 6 of `EVIDENCE_CLASSES` becomes `notYetReached` with the TG-8 property and question.
   - `scripts/ci/tier-gate.test.mjs`: the every-class test expects the new `evals/a.json` line.
3. Prose, in one commit: the new explainer and its row in `docs/gates/README.md`; the `evals/**` row and its bullet in `TIER-LAYER-MAP.md`; `evals/README.md`; stage 5's explainer sentence, stage 6 and the stage table in `DEVELOPMENT-FRAMEWORK.md`; the chain in `CLAUDE.md`; pointer lines in the two earlier specs (DOC-7).
4. Set PB-1.12 to `done` in `docs/TASKS.md` with this intent as its record, and add a root `JOURNAL.md` entry.

## Files

| File | Protected | Change |
|---|---|---|
| `scripts/ci/incident-eval-check.mjs` | yes (`scripts/ci/**`) | message, link, comment |
| `scripts/ci/incident-eval-check.test.mjs` | yes | expected message, two rebase-merge tests |
| `scripts/ci/tier-gate.mjs` | yes | row 6 of `EVIDENCE_CLASSES` |
| `scripts/ci/tier-gate.test.mjs` | yes | one expected report line |
| `docs/assurance/TIER-LAYER-MAP.md` | yes (`docs/assurance/**`) | evidence row and bullet |
| `docs/assurance/DEVELOPMENT-FRAMEWORK.md` | yes | stage table, stage 5 sentence, stage 6 |
| `evals/README.md` | yes (`evals/**`) | "CI linkage" section |
| `docs/gates/incident-eval-check.md` | no | new explainer |
| `docs/gates/README.md` | no | gate 18 |
| `CLAUDE.md` | no | chain |
| `intent/2026-09-30-incident-eval-range-spec.md`, `intent/2026-09-30-tier-anchor-spec.md` | no | IE-2 and row 6 point here |
| `docs/TASKS.md`, `JOURNAL.md` | no | record |

## Risks

- **Merge conflicts with open pull requests.** #63 and #64 change `scripts/ci/tier-gate.mjs`, and #63 changes `TIER-LAYER-MAP.md` and `DEVELOPMENT-FRAMEWORK.md`. This change touches one row of `EVIDENCE_CLASSES` and separate paragraphs of the two documents, so a conflict, if any, is textual and local.
- **The rebase-merge test simulates GitHub.** It replays the commits with a different committer, as GitHub's documentation says a rebase merge does. It does not run on GitHub, and the ruleset forbids rebase merges, so no real run can confirm it.
- **Self-trigger.** The check reads this pull request's body and commits after the merge. They must not contain the trigger text with its colon. The explainer and the specs may, because the check reads no file for the trigger.

## Proof that it worked

- `node --test scripts/ci/*.test.mjs` passes, including the two IE-6 rebase-merge tests and the exit-1 tests that assert the whole IE-8 output.
- `node scripts/ci/tier-gate.mjs` on a changed `evals/` file prints the TG-8 row 6 line.
- Running the check locally against merged #62 with the command in the explainer reproduces that run's exit 1 and prints the new message and link.
- `node scripts/ci/task-queue.mjs check` passes with `Task: PB-1.12`, and the Tier Gate passes at Tier 3.
