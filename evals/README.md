# evals/

This directory holds the eval suite that grows out of production incidents. Each production incident gets an eval that stays in the suite as a regression test, so the same failure cannot recur unnoticed. Evals also run when `CLAUDE.md`, a skill, or a hook changes, since those are the surfaces most likely to shift agent behaviour silently.

## Naming convention

`evals/<incident-or-behaviour>.md` for a written eval (scenario, expected behaviour, and how to check it), or an executable format (e.g. a script or fixture file) with the same base name where one already exists for that incident or behaviour. Keep the name descriptive rather than referencing a ticket number alone, so the eval is legible without cross-referencing another system.

## CI linkage

`incident-eval-check.yml` applies to a merged pull request that has the `incident` label, or a `Fixes-Incident: <id>` line in its body or in one of its commit messages. It needs both of these, each containing the incident id:

- an eval under `evals/`, matched by its path or its content;
- a candidate invariant under `docs/invariants/` or `crosscheck/docs/invariants/`, matched by its content.

If either is missing, the check fails. It runs only after the pull request is merged, so it reports on the merge and cannot block it. Add the eval and the invariant in the pull request that fixes the incident. The check finds the file and does not run the eval. The full explanation is `docs/gates/incident-eval-check.md`.

## Full picture

See `docs/assurance/DEVELOPMENT-FRAMEWORK.md` for how incident records and evals feed back into a new intent file at stage 1, closing the loop from Maintain back to Plan.
