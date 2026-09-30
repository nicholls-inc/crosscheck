# Plan: Read a merged pull request's commits in the Incident Eval Check

Intent: `intent/2026-09-30-incident-eval-range.md`
Spec: `intent/2026-09-30-incident-eval-range-spec.md` (IE-1 to IE-7)
Governing roadmap item: PB-1. Task: PB-1.2. Tier: 3.

This plan is not the root `plan.md`, which belongs to an earlier change. The pull request body cites this file with a `Plan:` line.

## Order of work

1. Write the governance note `.assurance/protected-surface-amend/incident-eval-range-2026-09-30.md`, naming the three protected files below.
2. Write `scripts/ci/incident-eval-check.test.mjs` (IE-6). Run it and see the squash-merge cases fail against the current script.
3. In `scripts/ci/incident-eval-check.mjs`, replace the `COMMIT_MESSAGES` input with a function that fetches `refs/pull/<n>/head` and runs `git log -z --format=%B origin/<base>..<head>` through `execFileSync` (IE-1 to IE-3, IE-5).
4. Collapse the two workflow steps into one (IE-4).
5. Run `node --test scripts/ci/*.test.mjs`.
6. In a fresh clone of `origin`, run the script for #53 and #46, whose runs failed, and check that it prints the skip line and exits 0.
7. Set PB-1.2 to `done` in `docs/TASKS.md`, with this intent as its record.

## Files

| File | Protected | Change |
|---|---|---|
| `.github/workflows/incident-eval-check.yml` | yes (`.github/workflows/**`) | one step, inputs as env (IE-4) |
| `scripts/ci/incident-eval-check.mjs` | yes (`scripts/ci/**`) | collects the commits itself (IE-1 to IE-3, IE-5) |
| `scripts/ci/incident-eval-check.test.mjs` | yes (`scripts/ci/**`) | new (IE-6) |
| `docs/TASKS.md` | no | PB-1.2 `done` |
| `.assurance/protected-surface-amend/incident-eval-range-2026-09-30.md` | no | new governance note |
| `intent/2026-09-30-incident-eval-range*.md` | no | stage artefacts |

## Risks

- **GitHub stops keeping `refs/pull/<n>/head`.** The fetch then fails and the check exits 2, which is visible. It does not pass silently.
- **A fork pull request.** Its head ref is still published as `refs/pull/<n>/head` on the base repository, so the fetch is the same.
- **The fix is proved only after merge.** The workflow runs on `pull_request: closed`, so its first real run is this pull request's own merge. The scratch-remote tests and step 6 are the evidence before that.

## Proof that it worked

- `node --test scripts/ci/*.test.mjs` passes, including the twenty-one IE-6 cases.
- Step 6 prints `no incident reference — skipped` for #53 and #46.
- After merge, the `Incident Eval Check` run for this pull request succeeds. If it does not, PB-1.2 goes back to `todo` (intent, "After merge").
