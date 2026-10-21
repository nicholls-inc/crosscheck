# Intent: Reorder the queue, set item statuses, and correct the kernel-replay statement

Governing roadmap item: PB-1. Task: PB-1.53. Spec: `intent/2026-10-21-queue-housekeeping-spec.md`. Plan: `intent/2026-10-21-queue-housekeeping-plan.md`.

## Authority
The maintainer, in chat on 2026-10-21, told the coordinating agent to do this housekeeping, including changing roadmap item statuses and adding queue rows, and to ship it without asking: "do the housekeeping. I give you explicit permission to merge PRs and ship the improvements and housekeeping tasks." The maintainer chose the order of the queue and the text of the five new rows.

## Problem statement
- The queue's order does not match the maintainer's priorities. The first open row is PB-1.17. The maintainer wants CGV's reach on real code and its link to Crosscheck first.
- Five pieces of work have no row: a `cgv_check` MCP tool (ER-1.20), CGV named as Layer 3 reach in three onboarding skills (VA-1.19), `byfuglien` routing to `cgv_check` (ER-1.21), the first worked example of RQ-1 (RQ-1.1), and a rerun of the real-codebase evaluation (CG-1.51).
- `docs/assurance/ROADMAP.md` says VA-1, ER-1, CG-1, TB-1 and AD-1 are "Not started", though each has merged tasks.
- `crosscheck/docs/assurance-hierarchy.md` (Layer 3 row) and `crosscheck/README.md` say a kernel replay of the built environment is not yet reached and ask whether it fits CI's time budget. TB-1.1 (#99, merged 2026-10-07) added the replay to CGV CI.
- Some open issues may have no work left.

## Proposed outcome
- `docs/TASKS.md`: the first open rows are, in order, CG-1.28, CG-1.35, ER-1.20, TB-1.14, VA-1.19, CG-1.29, CG-1.14, CG-1.37, ER-1.21, RQ-1.1, CG-1.51. Every other row keeps its relative order. PB-1.53 records this work as done.
- `docs/assurance/ROADMAP.md`: VA-1, ER-1, CG-1, TB-1 and AD-1 are `In progress`. RQ-1 names RQ-1.1. No item's scope or acceptance changes.
- The two Crosscheck docs say CGV CI replays the kernel, state the replay's limits from CI-9, and keep "not yet reached" for a second, independent checker.
- Each open issue whose work a merged pull request completed, and whose queue rows are all done, is closed with a comment that names the pull request. This is a GitHub action, not part of the diff.

## Affected users and systems
- Agents that run "pick up next task": `node scripts/ci/task-queue.mjs next` prints CG-1.28 after the merge.
- Readers of the roadmap and of the two Crosscheck docs.
- `docs/assurance/ROADMAP.md` is a protected surface, so the pull request is Tier 3 with a governance note.

## Constraints
- Rows keep the queue's style: "not yet reached" with the blocking property and the open question, and never "excluded" or "out of scope" (`docs/VISION.md`).
- New IDs are the next free number under each item: PB-1.53, VA-1.19, ER-1.20, ER-1.21, RQ-1.1, CG-1.51.
- The only row newly set to `done` is PB-1.53, so the task queue check passes with `Task: PB-1.53`.
- `crosscheck/README.md` and `crosscheck/docs/assurance-hierarchy.md` are not behavioural artefacts, so `docs:` is the commit type.

## Decided without the maintainer
- **ER-1's status.** Each acceptance clause has a merged pull request: CGV emits a record (ER-1.2), `/generate-verified` emits one (ER-1.6, #91), and the checker rejects a claim with no strength or no rerun command (ER-1.4). But ER-1.12 to ER-1.19 are open, and this pull request adds ER-1.20 and ER-1.21. The status is `In progress`, not `Done`. The maintainer decides whether ER-1 meets its acceptance.
- **RQ-1's status.** No RQ-1 task is done, so RQ-1 stays `Not started`.
- **Issues for the new rows.** No issue is opened. The rows hold the scope, and an issue can be opened when the work starts.
- **Issues with open rows stay open.** #47, #48, #80, #7, #9 and #36 each have a merged pull request, but each also has a `todo` row that names it. #10 waits on #74, as its own comments say. #27 is the tracker for roadmap item AD-1, whose tasks are open. #81 and #86 were found during ER-1.3 and CG-1.7, and no pull request did their work.
- **Placement of PB-1.53.** It goes after PB-1.52, with the other PB rows. Done rows do not affect `next`.

## Open questions
None.
