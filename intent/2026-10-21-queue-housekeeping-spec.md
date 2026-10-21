# Spec: Queue housekeeping of 2026-10-21

Intent: `intent/2026-10-21-queue-housekeeping.md`. Each rule is checked by the command under it.

- **QH-1.** The first 11 rows of `docs/TASKS.md` whose status is not `done` are, in order, CG-1.28, CG-1.35, ER-1.20, TB-1.14, VA-1.19, CG-1.29, CG-1.14, CG-1.37, ER-1.21, RQ-1.1, CG-1.51.
- **QH-2.** Every row on the base branch is still in the queue with the same text, and the rows that QH-1 does not name keep their relative order. The queue gains exactly six rows: PB-1.53, ER-1.20, VA-1.19, ER-1.21, RQ-1.1, CG-1.51.
- **QH-3.** The only row newly `done` is PB-1.53. CG-1.51 is `blocked`, and says what unblocks it. ER-1.21 depends on ER-1.20 and VA-1.19.
- **QH-4.** `node scripts/ci/task-queue.mjs check` passes with `BASE_REF=main` and `PR_BODY` holding `Task: PB-1.53`. The queue's `next` rule, applied to the branch's queue and the claimed `task/*` branches, gives CG-1.28.
- **QH-5.** In `docs/assurance/ROADMAP.md`, VA-1, ER-1, CG-1, TB-1 and AD-1 read `Status: In progress`. PB-1 and MR-1 are unchanged. RQ-1, CE-1, SC-1 and CL-1 stay `Not started`. RQ-1 names its task RQ-1.1. No scope or acceptance sentence changes.
- **QH-6.** No file under `crosscheck/` except `crosscheck/JOURNAL.md` says a kernel replay of the built environment is not yet reached, or asks whether a replay fits CI's time budget. The journal is a historical record and is left as written.
- **QH-7.** `docs/assurance/ROADMAP.md` is the only protected path that the branch changes, and the governance note `.assurance/protected-surface-amend/queue-housekeeping-2026-10-21.md` names it.

## Concerns flagged
- ER-1's acceptance clauses each have a merged pull request, but the item has open rows. The spec keeps it `In progress` and leaves `Done` to the maintainer.
- QH-6 is checked by a text search, so it finds only the wording it looks for. A reworded stale statement would pass it.
