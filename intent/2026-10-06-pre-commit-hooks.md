# Intent: Pre-commit hooks for the checks that need no PR body

Task: PB-1.9. Governing roadmap item: PB-1.

## Problem statement
The roadmap's dual-track principle (`docs/assurance/ROADMAP.md`, "Dual-track enforcement principle") says every deterministic check gets two enforcement points: a pre-commit hook that runs in under 5 seconds and prints the command that fixes the failure, and a CI job. Two checks have only the CI job.

1. **The tier gate** (`scripts/ci/tier-gate.mjs`, TG-5). A change to a protected path must come with a governance note, changed on the same branch, that names the path. An author learns this only after pushing and opening a pull request. The PreToolUse hook in `.claude/hooks/` catches the same mistake, but only for edits made through Claude Code's edit tools. An edit made with a shell command, another agent, or a human editor reaches the commit unchecked.
2. **The task queue check** (`scripts/ci/task-queue.mjs check`, QC-1 to QC-4). A malformed task ID, an unknown roadmap item, a repeated ID, an unknown status or a dangling dependency in `docs/TASKS.md` is also caught only in CI.

The only hook in `.husky/` today is `commit-msg`.

## Proposed outcome
- `.husky/pre-commit` runs `node scripts/ci/pre-commit.mjs`. `npm install` at the repository root installs it, as it already installs `commit-msg`.
- The hook reads the commit as it will be made, from the index, and blocks the commit when:
  - the commit stages a protected path, and some protected path changed on the branch (the index against the merge base with the default branch) is not named in a governance note changed on the branch. This is TG-5, with the same note rule as CI.
  - the commit stages `docs/TASKS.md` or `docs/assurance/ROADMAP.md`, and the staged queue fails QC-1 to QC-4.
- Each failure names the problem, prints the command that fixes it, and prints the command that reruns the hook's checks.
- The rules that need the PR body stay in CI only: the tier declaration (TG-1), the citations (TG-2 to TG-4), the CGV proof-surface section (TG-7) and the `Task:` line (QC-5).
- A test commits through the real hook in a scratch repository and checks each case and the 5-second budget.

## Affected users and systems
- Every author who commits in this repository after running `npm install` at the root: humans, Claude Code, and other agents.
- `scripts/ci/tier-gate.mjs` and `scripts/ci/task-queue.mjs` export the pieces the hook reuses. Their CI behaviour does not change.
- New: `.husky/pre-commit`, `scripts/ci/pre-commit.mjs`, `scripts/ci/pre-commit.test.mjs`.
- `docs/assurance/DEVELOPMENT-FRAMEWORK.md`, `docs/assurance/TIER-LAYER-MAP.md`, `docs/gates/tier-layer-gate.md` and `docs/gates/task-queue-check.md` say the hook exists and what it checks.

## Constraints
- Under 5 seconds, no network, no LLM. The hook never fetches. It reads the default branch as last fetched, like the PreToolUse hook.
- No dependency is added.
- The CI jobs stay the authority. The hook checks a subset of their rules and must never pass a commit that the same rule fails in CI, given the same base.
- A commit that touches neither a protected path nor the queue files runs no git command beyond reading the staged file list, so a broken queue on the default branch, or a clone with no `origin`, does not block unrelated commits.
- `git commit --no-verify` skips the hook. CI still runs every rule.

## Open questions
None. The task row and the dual-track principle state the outcome, and the choice of which rules need no PR body follows from the rules' inputs.
