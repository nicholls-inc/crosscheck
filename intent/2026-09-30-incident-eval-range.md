# Intent: Read a merged pull request's commits in the Incident Eval Check

Task: PB-1.2. Governing roadmap item: PB-1.

## Problem statement
The `Incident Eval Check` workflow has failed on every merged pull request since #43. Runs 36596017303, 36596783609, 36606048674, 36612206524, 36734821509 and 36735093019 all end the same way:

```
fatal: Invalid revision range origin/main..c80201c68cb2c94ce5b9afae9bdeaea0f3a05660
##[error]Invalid value. Matching delimiter not found 'EOF'
```

The step runs `git log origin/main..<head.sha>` after the merge. The default branch allows only squash merges, and the head branch is deleted on merge. So the pull request's own commits are on no branch, `actions/checkout` does not fetch them, and git cannot resolve `head.sha`. #42 passed only because it was a merge commit, which puts the head commits on `main`.

When `git log` fails, the step never writes the closing `EOF` line to `$GITHUB_OUTPUT`, so the runner reports a second, misleading error. The check has not run its incident rules on any squash-merged pull request.

GitHub keeps `refs/pull/<number>/head` after the branch is deleted. `git ls-remote origin 'refs/pull/*/head'` on 2026-09-30 lists it for #43, #46 and #53, whose branches no longer exist.

## Proposed outcome
- The check fetches `refs/pull/<number>/head` and reads the commits in `origin/<base>..<head.sha>`. It works whether the pull request was squash-merged or merge-committed, and whether or not the branch still exists.
- If git cannot produce the commits, the check fails with the git command and its error. It never reports "no incident reference — skipped" for commits it did not read.
- The commit collection lives in `scripts/ci/incident-eval-check.mjs`, so `node --test scripts/ci/*.test.mjs` in the Tier Gate job tests it against a scratch remote that reproduces a squash merge with a deleted branch.

## Affected users and systems
- The maintainer, who reads the check's result on each merge.
- `.github/workflows/incident-eval-check.yml` and `scripts/ci/incident-eval-check.mjs`.
- The Tier Gate job runs one more test file.

## Constraints
- The incident rules do not change: the `incident` label, the `Fixes-Incident:` line, and the eval and invariant lookups keep their meaning and their exit code 1.
- The range stays `origin/<base>..<head.sha>`, not `base.sha..head.sha`. After the merge, `origin/<base>` contains every commit of the base branch that was merged into the pull request's branch, so those commits are excluded and cannot trigger the check.
- No CI job calls an LLM.

## Open questions
None.
