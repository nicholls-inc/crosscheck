# Spec: Pre-commit hooks for the checks that need no PR body

Intent: `intent/2026-10-06-pre-commit-hooks.md`. Governing roadmap item: PB-1. Task: PB-1.9.

This spec adds PC-1 to PC-8. The TG requirements (`intent/2026-09-29-deterministic-evidence-spec.md` and its revisions) and the QC requirements (`intent/2026-10-01-queue-check-spec.md`) do not change, and neither does the result of either CI job on any pull request.

## Terms

- **Staged set.** The paths in `git diff --cached --name-only --no-renames -z`: what this commit changes against `HEAD`.
- **Default branch.** `origin/HEAD`, else `origin/main`, as last fetched. The hook never fetches. This is the PreToolUse hook's rule.
- **Branch set.** The paths in `git diff --cached --name-only --no-renames -z --merge-base <default branch>`: what the pull request will change once this commit is pushed. `--merge-base` needs git 2.30 or later.
- **Protected path.** A path that matches a glob in the machine-readable list of `.claude/rules/protected-surfaces.md`, parsed and matched as `scripts/ci/tier-gate.mjs` does.
- **Governance note.** A path that `tier-gate.mjs` treats as one (`.assurance/protected-surface-amend/*.md` or `.assurance/add-session-*/**.md`).

`-z` keeps a path with a byte outside printable ASCII unquoted. Task PB-1.10 fixes the same fault in the CI job.

## Requirements

- **PC-1. The hook.** `.husky/pre-commit` runs `node scripts/ci/pre-commit.mjs` from the repository root, and the commit fails when it exits non-zero. Husky installs it on `npm install` at the root, through the existing `prepare` script.
- **PC-2. Exit codes.** `pre-commit.mjs` exits 0 when every check that applies passes, 1 when a check finds a problem, and 2 when it cannot read an input it needs. It runs every check that applies before it exits, and prints every problem.
- **PC-3. Governance notes (TG-5 at commit time).** The check applies when the staged set has a protected path.
  - It computes the branch set, and the protected paths in it.
  - The note text is the text, read from the index, of every governance note in the branch set that is present in the index. A note deleted on the branch is not read.
  - It fails when a protected path in the branch set appears in no note text as a substring. This is the TG-5 rule, so a note on the default branch that this branch has not changed does not count.
  - It exits 2 when the rules file has no machine-readable list, or when neither `origin/HEAD` nor `origin/main` resolves. The fix it prints for the second case is `git fetch origin`.
- **PC-4. The queue (QC-1 to QC-4 at commit time).** The check applies when the staged set has `docs/TASKS.md` or `docs/assurance/ROADMAP.md`.
  - It reads both files from the index.
  - It fails on the problems QC-1 to QC-4 report: a task ID not of the form `<item ID>.<n>`, an item that the roadmap does not define, a repeated task ID, an unknown status, and a dependency that names no row.
  - It does not apply QC-5, because the `Task:` line is in the PR body. A row newly set to `done` passes.
  - It exits 2 when the queue table has no separator row (QP-4), and fails when `docs/TASKS.md` has no queue. It also exits 2 when either file is staged as a deletion, because it cannot read it from the index, and the `Fix:` it prints is `git restore --staged <path>`.
- **PC-5. Messages.** For each failing check the hook prints the problems, one per line, then a line that starts `Fix:` and names a command, then the rerun command `node scripts/ci/pre-commit.mjs`, then the link to the check's explainer under `docs/gates/`.
  - Governance notes: `Fix:` names `/crosscheck:protected-surface-amend` and `git add .assurance/protected-surface-amend/<note>.md`, and, for a file staged by mistake, `git restore --staged <path>` with the unnamed staged paths.
  - Queue: `Fix:` names the edit to `docs/TASKS.md` that each problem line states, then `git add docs/TASKS.md`.
  - Base unresolved: `Fix: git fetch origin`.
  - Any other input the hook cannot read: `Fix:` tells the user to run the git command printed in the problem line by hand and resolve its error.
- **PC-6. Applies only when needed.** A commit whose staged set has no protected path, no `docs/TASKS.md` and no `docs/assurance/ROADMAP.md` passes after reading the staged set and the rules file, and reads nothing else. It does not need `origin`.
- **PC-7. Budget.** No network, no LLM, and under 5 seconds on this repository. The test asserts the 5 seconds on each commit it makes. It times the hook process alone, from git's trace2 `child_start` and `child_exit` events for the `pre-commit` hook (`GIT_TRACE2_EVENT`), so git's own work and the test's setup are not counted. The assertion message states the measured milliseconds.
- **PC-8. Tests.** `scripts/ci/pre-commit.test.mjs` copies `.husky/pre-commit`, `scripts/ci/pre-commit.mjs`, `scripts/ci/tier-gate.mjs`, `scripts/ci/task-queue.mjs` and the rules file into a scratch clone of a scratch bare remote, sets `core.hooksPath` to `.husky`, and runs `git commit`. `git commit` exits 1 for any failing hook, so each case asserts whether the commit failed, whether `HEAD` moved, the exit code of `node scripts/ci/pre-commit.mjs` rerun in the same clone, and literal text in the output:
  - a staged protected path with no note fails, names the path, and prints `Fix:` with `/crosscheck:protected-surface-amend` (PC-3, PC-5);
  - the same commit with a new note that names the path passes (PC-3);
  - a note committed earlier on the branch counts (PC-3);
  - a note deleted on the branch, though it is on the default branch, is not read (PC-3);
  - a note on the default branch that names the path, unchanged on the branch, does not count (PC-3);
  - a staged protected path with no `origin/HEAD` or `origin/main` exits 2 and prints `Fix: git fetch origin` (PC-3);
  - a staged queue with an unknown status fails and names the row (PC-4);
  - a staged queue that sets a row to `done` with no `Task:` line passes (PC-4);
  - a staged queue with no separator row exits 2 (PC-4);
  - a staged `docs/TASKS.md` with no queue fails (PC-4);
  - a queue file staged as a deletion exits 2 and prints `Fix: git restore --staged <path>` (PC-4, PC-5);
  - an unrelated commit in a clone with no `origin` passes (PC-6);
  - on each commit, the pre-commit hook process runs for under 5 seconds, as git's trace2 records it (PC-7).

## Evidence report

`EVIDENCE_CLASSES` in `tier-gate.mjs` and the evidence table in `docs/assurance/TIER-LAYER-MAP.md` map `.husky/pre-commit` to the Tier Gate workflow, whose `node --test scripts/ci/*.test.mjs` runs PC-8. `.husky/commit-msg` has no test and stays "not yet reached".

## Concerns flagged, not resolved here

- The hook runs only where `npm install` ran at the root, and `--no-verify` skips it. It is a fast first check, not evidence. CI remains the enforcement point that every pull request passes through.
- The hook reads the default branch as last fetched. A stale `origin/main` gives a different branch set from CI's. It can then pass a commit that CI fails, or fail one that CI passes. `git fetch origin` makes them agree.
- The governance-note rule is a substring match, as in CI. A note that names `docs/assurance/ROADMAP.md.bak` also names `docs/assurance/ROADMAP.md`. This is TG-5's behaviour and is not new.
