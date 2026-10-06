# Intent: Stop the tier gate's explainer claiming it checks incident evals

Task: PB-1.11. Governing roadmap item: PB-1.

## Problem statement
`docs/gates/tier-layer-gate.md` has a section, "What the incident-eval check adds", which says that when a pull request fixes an incident, "the tier gate additionally expects" an eval under `evals/`, and that the author must "add one before the gate will pass". Both claims are wrong, and a newcomer can check them:

1. The tier gate reads no incident reference. `scripts/ci/tier-gate.mjs` matches `evals/` only to list which job holds the evidence for a changed file. A run with `PR_LABELS=incident`, a body of `Tier: 1`, an `Intent:` line, an incident id that no file under `evals/` or `docs/invariants/` names, and `CHANGED_FILES=crosscheck/README.md` exits 0.
2. A separate workflow, `.github/workflows/incident-eval-check.yml`, checks incident evals. It runs on `pull_request` `closed`, only when the pull request was merged, so it reports on a merge that has already happened. It also needs a candidate invariant, which the section does not mention.

A contributor who trusts the section waits for the tier gate to ask for an eval. It never does, and the first signal is a red Incident Eval Check after the merge.

## Proposed outcome
The section says that the tier gate does not check incidents, names the Incident Eval Check, states its trigger, the two files it needs, and that it runs after the merge and cannot block it. It points to stage 5 of `docs/assurance/DEVELOPMENT-FRAMEWORK.md` for the exit codes. No other section of the file changes.

## Affected users and systems
- A contributor who opens a pull request that fixes an incident.
- `docs/gates/tier-layer-gate.md`, `docs/TASKS.md` and `JOURNAL.md`. None is protected, so the change is Tier 1.

## Constraints
- The tier gate, the Incident Eval Check and their workflows do not change. The document follows the code.
- PB-1.12 owns the Incident Eval Check's own explainer under `docs/gates/` and its failure message. This change does not write that explainer.

## Open questions
None. The task row states the outcome, and the code states the behaviour.

The Incident Eval Check matches its trigger anywhere in a line, so prose that quotes the trigger also fires it. The run for #62, whose body and commits described the check, failed after the merge with the id `<id>` followed by a backtick. This change adds row PB-1.16 for that and does not fix it.
