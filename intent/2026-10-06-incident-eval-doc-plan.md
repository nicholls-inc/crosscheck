# Plan: State what the Incident Eval Check really does

Intent: `intent/2026-10-06-incident-eval-doc.md`
Governing roadmap item: PB-1. Task: PB-1.8. Tier: 3.

This plan is not the root `plan.md`, which belongs to an earlier change. The pull request body cites this file with a `Plan:` line. The change is prose that follows existing code, so it has no spec. The existing spec of the check, `intent/2026-09-30-incident-eval-range-spec.md` (IE-1 to IE-7), is the source for every claim.

## Order of work

1. Write the governance note `.assurance/protected-surface-amend/incident-eval-doc-2026-10-06.md`, naming `docs/assurance/DEVELOPMENT-FRAMEWORK.md`.
2. In stage 5 of `docs/assurance/DEVELOPMENT-FRAMEWORK.md`, replace the `incident-eval-check.yml` bullet. The new bullet states:
   - the trigger: the `incident` label, or a `Fixes-Incident: <id>` line in the body or a commit;
   - exit 1: no eval under `evals/` or no candidate invariant under `docs/invariants/` or `crosscheck/docs/invariants/` names the id, or the label is set with no id;
   - exit 2: the check cannot read the pull request's commits;
   - the workflow runs only after a merge, so it reports on the merge and cannot block it.
3. Set PB-1.8 to `done` in `docs/TASKS.md`, with this intent as its record. Add row PB-1.11 for the same wrong claim in `docs/gates/tier-layer-gate.md`. Add a root `JOURNAL.md` entry.

## Files

| File | Protected | Change |
|---|---|---|
| `docs/assurance/DEVELOPMENT-FRAMEWORK.md` | yes (`docs/assurance/**`) | one bullet in stage 5 |
| `docs/TASKS.md` | no | PB-1.8 `done`, new row PB-1.11 |
| `JOURNAL.md` | no | entry |
| `.assurance/protected-surface-amend/incident-eval-doc-2026-10-06.md` | no | new governance note |
| `intent/2026-10-06-incident-eval-doc*.md` | no | stage artefacts |

## Risks

- **The bullet could drift from the code again.** No check reads prose. The tier gate lists `docs/assurance/**` as "not yet reached", and this change does not alter that.
- **Exit 2 wording.** The bullet names the cause of exit 2 by its effect, "cannot read the commits", and leaves the list of causes to IE-2, so a new cause in the script does not make the bullet wrong.

## Proof that it worked

- Each claim in the new bullet maps to a line of `scripts/ci/incident-eval-check.mjs` or `.github/workflows/incident-eval-check.yml`, and to a passing case of `node --test scripts/ci/incident-eval-check.test.mjs` (IE-2, IE-5).
- `gh run list --workflow incident-eval-check.yml` shows the run for merged #61 start four seconds after its merge, and the run for unmerged #60 skipped.
- `node scripts/ci/task-queue.mjs check` passes with this pull request's `Task: PB-1.8` line, and this pull request's Tier Gate passes at Tier 3.
