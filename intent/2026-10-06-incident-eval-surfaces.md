# Intent: Describe the Incident Eval Check the same way everywhere

Task: PB-1.12. Governing roadmap item: PB-1.

## Problem statement
PB-1.8 rewrote the `incident-eval-check.yml` bullet in stage 5 of `docs/assurance/DEVELOPMENT-FRAMEWORK.md`. That bullet is now right: the check runs only after a pull request is merged, applies on the `incident` label or a `Fixes-Incident: <id>` line in the body or a commit, needs an eval under `evals/` and a candidate invariant under `docs/invariants/` or `crosscheck/docs/invariants/` that contain the id, and exits 2 when it cannot read the commits. Six other places still describe the check the old way, and a newcomer can check each one against `scripts/ci/incident-eval-check.mjs` and `.github/workflows/incident-eval-check.yml`:

1. **The failure message.** `printFailure` in `scripts/ci/incident-eval-check.mjs` says "declining means the change stays blocked until both artefacts are added". The message prints after the merge, so nothing is blocked. Its `Full explanation:` link points at `docs/gates/README.md`, the gate index, because the check has no explainer. Every other gate's link points at its own explainer (`docs/gates/README.md`, first paragraph).
2. **The evidence table.** `docs/assurance/TIER-LAYER-MAP.md` lists the Incident Eval Check as the deterministic evidence for `evals/**`, and `scripts/ci/tier-gate.mjs` prints "Incident Eval Check workflow" for a changed eval (TG-8 row 6 in `intent/2026-09-30-tier-anchor-spec.md`). The check runs no eval. It reads `evals/` only after a merge, and only to find a file that contains an incident id. A pull request that changes an eval and references no incident is skipped, and an eval that no longer reproduces anything still passes.
3. **`evals/README.md`** asks for a candidate invariant only "where applicable", names only `docs/invariants/`, and says a pull request without an eval "fails this check" without saying the check runs after the merge. The script always needs the invariant, and also accepts one under `crosscheck/docs/invariants/`.
4. **Stage 6 and the stage table** of `DEVELOPMENT-FRAMEWORK.md` say "incident record + eval". The check also needs a candidate invariant. `CLAUDE.md` repeats the same chain.
5. **IE-2** in `intent/2026-09-30-incident-eval-range-spec.md` says that "a merge commit or a rebase merge" leaves an empty range and exits 2. GitHub's rebase merge "always updates the committer information and creates new commit SHAs" ([About merge methods on GitHub](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/configuring-pull-request-merges/about-merge-methods-on-github)). The pull request's head SHA is then not on the base branch, so `origin/<base>..<head>` holds the original commits and the check reads them. The script's comment repeats the claim, and no test covers a rebase merge.
6. **Stage 5 itself** says "The check has no explainer under `docs/gates/` yet. Task PB-1.12 adds one."

## Proposed outcome
- A new explainer, `docs/gates/incident-eval-check.md`, in the shape of the other explainers: what the check protects, when it runs, what it needs, its exit codes, what each decision means, what it does not catch, and how to run it locally. `docs/gates/README.md` lists it as gate 18.
- The failure message keeps the four-line gate shape. It asks for the eval and the invariant in a follow-up pull request, says what declining leaves behind instead of "stays blocked", and links to the new explainer.
- `evals/**` becomes "not yet reached" in the tier gate's report and in `TIER-LAYER-MAP.md`, with its blocking property and open question.
- `evals/README.md`, stage 6, the stage table and `CLAUDE.md` name the candidate invariant, and `evals/README.md` says the check runs after the merge.
- IE-2 and the script's comment say that a merge commit leaves an empty range and a rebase merge does not. A new test simulates GitHub's rebase merge and shows that the check reads the pull request's commits.

## Affected users and systems
- A contributor who fixes an incident, and the maintainer who reads the check's result after each merge.
- Anyone who reads the tier gate's pass report for a change under `evals/`.
- Protected (Class A): `scripts/ci/incident-eval-check.mjs`, `scripts/ci/incident-eval-check.test.mjs`, `scripts/ci/tier-gate.mjs`, `scripts/ci/tier-gate.test.mjs`, `docs/assurance/TIER-LAYER-MAP.md`, `docs/assurance/DEVELOPMENT-FRAMEWORK.md`, `evals/README.md`. The change is Tier 3.
- Not protected: `docs/gates/incident-eval-check.md`, `docs/gates/README.md`, `CLAUDE.md`, the two earlier specs, `docs/TASKS.md`, `JOURNAL.md`.

## Constraints
- The check's trigger, its exit codes and its workflow do not change. Only the failure message's second sentence and link change.
- The tier gate's pass or fail result does not change. Only the report line for `evals/**` changes.
- `docs/gates/tier-layer-gate.md` belongs to PB-1.11 (#65), and the trigger that matches quoted prose belongs to PB-1.13, proposed in #65. This change does not edit that file or the trigger, and the explainer states the current trigger behaviour.
- `crosscheck/skills/assurance-init/SKILL.md` and `docs/assurance/ROADMAP.md` also quote the chain as "incident record + eval". The skill scaffolds that chain into other repositories, whose incident job may differ, and the roadmap quotes the playbook's chain by name. Neither describes this repository's check, so neither changes here.
- The pull request body and its commit messages must not contain the trigger text with its colon, or the check will fire on this pull request's own merge, as it did on #62.

## Open questions
None. The task row states each outcome, and the code and GitHub's documentation state the behaviour.
