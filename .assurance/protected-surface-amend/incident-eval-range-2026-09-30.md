## Protected-Surface Amendment

**Target file(s):** `.github/workflows/incident-eval-check.yml`, `scripts/ci/incident-eval-check.mjs`, `scripts/ci/incident-eval-check.test.mjs`
**Class:** A (CI enforcement)
**Matched rule:** `.github/workflows/**`, `scripts/ci/**`
**Date:** 2026-09-30

### Change Description

1. `scripts/ci/incident-eval-check.mjs`: the `COMMIT_MESSAGES` input is replaced by `PR_NUMBER`, `BASE_REF` and `HEAD_SHA`. The script fetches `refs/pull/<PR_NUMBER>/head` and reads the messages in `origin/<BASE_REF>..<HEAD_SHA>` with `execFileSync`. Bad inputs, a git failure, or an empty range exit 2 with the cause. Commit messages are matched line by line, as before. `git remote get-url` also runs without a shell. The incident rules and their exit codes 0 and 1 are unchanged.
2. `.github/workflows/incident-eval-check.yml`: the "Collect PR commit messages" step and its `$GITHUB_OUTPUT` heredoc are removed. The remaining step passes the five inputs as environment variables.
3. `scripts/ci/incident-eval-check.test.mjs`: new. Twelve cases, ten of them against a scratch remote that reproduces a squash merge with a deleted head branch, and two that read the script and the workflow for IE-3 and IE-4.

### Rationale

The check has failed on every squash-merged pull request since #43 with `fatal: Invalid revision range origin/main..<head.sha>`. The default branch allows only squash merges and deletes the head branch, so `actions/checkout` never fetches the head commits. GitHub keeps `refs/pull/<n>/head`, so fetching it makes the range resolvable. Intent: `intent/2026-09-30-incident-eval-range.md`. Spec: `intent/2026-09-30-incident-eval-range-spec.md`. Plan: `intent/2026-09-30-incident-eval-range-plan.md`.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (immediate horizon, item PB-1)
- **Title:** Adopt the AI-native SDLC playbook as the development framework and make every human gate self-explanatory
- **Scope coverage:** PB-1's scope names `incident-eval-check.yml` as one of its deterministic CI jobs. Task PB-1.2 in `docs/TASKS.md` is this fix.

### Authority

- **Authoriser:** harry-nicholls. The merge is the approval.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `scripts/ci/incident-eval-check.mjs` | inputs, commit collection | replaced |
| 2 | `.github/workflows/incident-eval-check.yml` | steps | two steps collapsed into one |
| 3 | `scripts/ci/incident-eval-check.test.mjs` | whole file | added |

### Test / Coverage Impact

- `node --test scripts/ci/*.test.mjs`, run by the Tier Gate job, now includes the twelve IE-6 cases.
- The incident rules are unchanged, so no invariant or eval changes.

### Review Checklist

- [ ] The range is `origin/<base>..<head.sha>`, so base-branch commits merged into the pull request's branch are not read.
- [ ] A git failure exits 2 and never prints the skip line.
- [ ] No `${{ }}` expression is interpolated into a `run:` script.
- [ ] A merge that now exits 2 was a silent skip before. Exit 2 is expected after a merge commit or a rebase merge (empty range), and after GitHub stops publishing `refs/pull/<n>/head`; it is a visible red run, not a blocked merge, since the job runs after the merge.
- [ ] `fetch-depth: 0` stays: `origin/<base>..<head>` needs the base branch's history to exclude base commits merged into the branch. A fork pull request's head is published on this repository as `refs/pull/<n>/head`, so the same fetch reads it.
