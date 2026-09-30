# Spec: Read a merged pull request's commits in the Incident Eval Check

Intent: `intent/2026-09-30-incident-eval-range.md`. Governing roadmap item: PB-1.

- **IE-1.** `scripts/ci/incident-eval-check.mjs` reads three inputs from the environment: `PR_NUMBER`, `BASE_REF` and `HEAD_SHA`. It fetches `refs/pull/<PR_NUMBER>/head` from `origin` and reads the message of every commit in `origin/<BASE_REF>..<HEAD_SHA>`. It reads them after a squash merge whose head branch was deleted.
- **IE-2.** If `PR_NUMBER` is not a decimal number, `HEAD_SHA` is not 40 hex characters, `BASE_REF` is empty, or either git command fails, the check exits 2. It prints the git command and git's error. It does not print "no incident reference — skipped".
- **IE-3.** Git runs without a shell, with its arguments passed as an array.
- **IE-4.** `.github/workflows/incident-eval-check.yml` has one step after setup. It passes `PR_NUMBER`, `BASE_REF`, `HEAD_SHA`, `PR_BODY` and `PR_LABELS` as environment variables and runs `node scripts/ci/incident-eval-check.mjs`. It writes nothing to `$GITHUB_OUTPUT`, and its `run:` line interpolates no `${{ }}` expression.
- **IE-5.** The incident rules are unchanged. With no incident label and no `Fixes-Incident:` line in the body or the commits, the check exits 0. With either one, it exits 1 unless an eval and a candidate invariant reference the incident. The `COMMIT_MESSAGES` input is removed; the workflow was its only caller.
- **IE-6.** `scripts/ci/incident-eval-check.test.mjs` runs under `node --test scripts/ci/*.test.mjs`. Against a scratch remote, it covers:
  - a squash merge whose branch was deleted, with `Fixes-Incident:` only in a commit: exit 1, and the output names the incident;
  - the same, with no incident reference: exit 0;
  - a commit merged in from the base branch that carries `Fixes-Incident:`: it is not read, exit 0;
  - a `HEAD_SHA` that is absent from the remote: exit 2.
