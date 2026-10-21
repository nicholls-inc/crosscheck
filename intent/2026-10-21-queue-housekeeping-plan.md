# Plan: Queue housekeeping of 2026-10-21

Intent: `intent/2026-10-21-queue-housekeeping.md`. Spec: `intent/2026-10-21-queue-housekeeping-spec.md`.

## Files that change
1. `docs/TASKS.md`: add six rows and move six existing rows (QH-1 to QH-3).
2. `.assurance/protected-surface-amend/queue-housekeeping-2026-10-21.md`: the governance note, written before the roadmap edit (QH-7).
3. `docs/assurance/ROADMAP.md`: five `Status:` lines and one task reference (QH-5).
4. `crosscheck/docs/assurance-hierarchy.md` and `crosscheck/README.md`: the kernel-replay statement (QH-6).

## Order of work
1. Claim `task/PB-1.53`.
2. Move and add the rows with a script that fails if any other row changes order or if the row count is not the base count plus six.
3. Write the governance note, then edit the roadmap.
4. Edit the two Crosscheck docs.
5. Run the checks below, then close the issues that the intent's rule allows, each with a comment that names the pull request.

## Risks
- A moved row could lose text. The script compares the IDs of the rows it did not move before and after, and `git diff --word-diff` shows that moved rows are unchanged.
- A status change could read as an acceptance decision. Only `Not started` changes to `In progress`, and no item becomes `Done`.

## Proof
- `node --test scripts/ci/*.test.mjs` passes.
- `BASE_REF=main PR_BODY='Task: PB-1.53' node scripts/ci/task-queue.mjs check` passes.
- The first open rows of `docs/TASKS.md` are CG-1.28, CG-1.35, ER-1.20, TB-1.14, VA-1.19, CG-1.29, CG-1.14, CG-1.37, ER-1.21, RQ-1.1, CG-1.51, in that order, and `nextTask` from `scripts/ci/task-queue.mjs`, run on the branch's queue with no claims, gives CG-1.28. This proves row order, not live `next` output: this run's sibling lanes claimed `task/CG-1.28`, `task/CG-1.35`, `task/TB-1.14` and `task/CG-1.29`, so live `next` prints the first unclaimed row instead.
- `node scripts/ci/tier-gate.mjs` passes with the pull request body, `Tier: 3`, and the changed file list.
- `node scripts/ci/pre-commit.mjs` passes on each commit.
- `grep -rn "time budget" crosscheck --include='*.md'` finds no kernel-replay statement outside `crosscheck/JOURNAL.md`.
