# Intent: State what the Incident Eval Check really does

Task: PB-1.8. Governing roadmap item: PB-1.

## Problem statement
Stage 5 of `docs/assurance/DEVELOPMENT-FRAMEWORK.md` says that `incident-eval-check.yml` "fails when an incident record under `evals/` has no accompanying eval". Three parts of that sentence are wrong, and a newcomer can check each one against `scripts/ci/incident-eval-check.mjs` and `.github/workflows/incident-eval-check.yml`:

1. The check does not look for incident records under `evals/`. It applies when the pull request has the `incident` label, or a `Fixes-Incident: <id>` line in its body or in one of its commits. It then needs an eval under `evals/` and a candidate invariant under `docs/invariants/` or `crosscheck/docs/invariants/` that name the id, and exits 1 when either is missing. It also exits 1 when the label is set and no id is found.
2. The check has a second failure, exit 2, when it cannot read the pull request's commits: a malformed input, a git error, or an empty range after a merge commit (IE-2 in `intent/2026-09-30-incident-eval-range-spec.md`). It reads the commits before it looks for an incident reference, so this applies to every merged pull request. IE-2 also names a rebase merge, but GitHub's rebase merge writes new commit SHAs, which should leave the original commits in the range; no test covers it, and task PB-1.12 settles it. The document does not mention exit 2.
3. The workflow runs on `pull_request` `closed`, and only when the pull request was merged. It reports on a merge that has already happened. The section's lead says that no listed workflow can block a merge because the ruleset requires no status checks, but this check could not block one even with a required check, because it never runs on an open pull request. #61 merged at 17:59:08 UTC on 2026-10-06 and its Incident Eval Check run started at 17:59:12. The run for #60, closed without a merge, was skipped.

## Proposed outcome
The `incident-eval-check.yml` bullet in stage 5 names the trigger, the two artefacts, exit 1, exit 2, and that the workflow runs after the merge and cannot block it. No other line of the document changes.

## Affected users and systems
- A contributor who reads `DEVELOPMENT-FRAMEWORK.md` to learn what an incident pull request needs.
- The maintainer, who reads the check's result after each merge.
- `docs/assurance/DEVELOPMENT-FRAMEWORK.md` (protected, Class A), `docs/TASKS.md` and `JOURNAL.md`.

## Constraints
- The check and its workflow do not change. The document follows the code.
- `docs/assurance/**` is protected, so the change is Tier 3 and carries a plan and a governance note.

## Open questions
None. The task row states the outcome, and the code states the behaviour.

`docs/gates/tier-layer-gate.md`, under "What the incident-eval check adds", makes a related wrong claim. It says that the tier gate expects the eval "before the gate will pass". The tier gate never reads incident references, and the Incident Eval Check runs after the merge. That file is not in this task's row, so this change adds row PB-1.11 for it. Review found more places that describe the check the old way: its own failure message, `TIER-LAYER-MAP.md`, `evals/README.md`, and stage 6 of this document. Row PB-1.12 covers them, with the missing explainer and the rebase-merge question.
