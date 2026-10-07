# Intent: Review the imported Crosscheck backlog (#19 to #41) against the vision

Task: AD-1.1. Governing roadmap item: AD-1. Issue: #27.

## Problem statement
Issues #19 to #41 came into this repository with Crosscheck (MR-1). They were written before `docs/VISION.md` was agreed on 2026-09-29. Some ask for things the vision rules out. #22 asks for "an LLM judge that scores the transcript" as the pass condition of four acceptance oracles, and design rule 1 (`docs/VISION.md:48`) says no guarantee rests on the judgment of an LLM. Some are already done on `main`. Some are duplicates. None of them names the vision rule or the roadmap item it serves, and none has a row in `docs/TASKS.md`, so the pick-up procedure never reaches them.

## Proposed outcome
`intent/2026-10-07-backlog-review-decisions.md` records one decision for each of the 23 issues. Each decision is either Refine or Drop.

- **Refine.** The decision names the design rule or roadmap item the issue serves, gives the rewritten issue text, and names the `docs/TASKS.md` row that carries the work.
- **Drop.** The decision gives a reason that does not rest on an LLM's view of the issue's value. It is a duplicate, it is already done on `main` (the file and line that do it are cited), or it contradicts a cited line of `docs/VISION.md`. No reason puts a class of code beyond the vision's scope.

The decision table, with 21 Refine and 2 Drop:

- **#34 Drop.** Done on `main`, apart from two residues that move to AD-1.11.
- **#37 Drop.** Already done on `main`.
- **#27 Refine.** It becomes the tracker for AD-1, and its row is AD-1.1, this task.
- **The other 20 Refine.** They need 19 new rows, because #39 and #41 share AD-1.13, and #39, #40 and #41 share VA-1.11:
  - PB-1.18 to PB-1.21: deterministic CI checks of Crosscheck's own content;
  - VA-1.9 to VA-1.11: agents and skills that present an unchecked or LLM verdict as evidence;
  - AD-1.2 to AD-1.13: issues that serve a design rule but no other current item.

This pull request changes only these four files: the decisions file, this intent, `docs/TASKS.md`, and an entry in the root `JOURNAL.md`. It adds the 19 rows, and it sets AD-1.1 to `done` with this intent as its record. It edits and closes no GitHub issue. `done` on AD-1.1 means the decisions are recorded; applying them to the issues is the coordinator's step after review, and no queue row owns it. After review approves the head, the coordinator applies the recorded decisions to the issues: it rewrites each refined issue with its recorded text and closes each dropped issue with its recorded reason. The pull request is the record those edits follow. No work starts on any issue in this task.

## Affected users and systems
- The maintainer, who decides by merging whether each decision stands.
- Agents that run "pick up next task". The new rows enter the queue as `todo`.
- `docs/TASKS.md`, `JOURNAL.md` and two files under `intent/`. None is a protected surface, so the change is Tier 1. Most new rows edit Class A or Class B surfaces when they are picked up, and each such row says so.

## Constraints
- Task IDs are never reused. Before choosing IDs, the rows on `origin/main` and the `docs/TASKS.md` diff of every open pull request were read on 2026-10-07. They use PB-1 up to PB-1.17, VA-1 up to VA-1.8, ER-1 up to ER-1.17, TB-1 up to TB-1.4, and AD-1.1. The new rows start above each of those.
- A row goes under the roadmap item its work serves. Thirteen refined issues carry work that serves a design rule of the vision and no other current item. That work goes in twelve rows under AD-1, the item that governs this backlog, and each row names the rule it serves. Only the maintainer approves a new roadmap item. If the maintainer opens one, for example a CL-1 item for the vision's Tests class, the row moves under it.
- Facts about `main` were checked on the head of `origin/main` at `e5089f7`. The local clone is shallow, so a fix older than the import commit `06b6d25` cannot be traced to its commit. A "done on `main`" reason cites the file and line instead.
- The vision rule on scope applies to every decision and row. A class of code that a tool does not reach is "not yet reached", with the property that blocks it and the open question.

## Open questions
None for this task. Each decision is the maintainer's to overturn at review. Each refined row carries its own open questions into its own intent. For example, AD-1.2 decides where the auditor writes its report.
