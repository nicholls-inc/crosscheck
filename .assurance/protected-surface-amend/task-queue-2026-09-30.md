## Protected-Surface Amendment

**Target file(s):** `docs/assurance/ROADMAP.md`, `docs/assurance/DEVELOPMENT-FRAMEWORK.md`
**Class:** A (governance and roadmap documents the gates read)
**Matched rule:** `docs/assurance/**`
**Date:** 2026-09-30

### Change Description

1. `docs/assurance/ROADMAP.md` gains nine items, all with status `Not started`: VA-1, ER-1 and CG-1 in the Next horizon, TB-1, RQ-1 and AD-1 in the Medium-term horizon, and CE-1, SC-1 and CL-1 in the Aspirational horizon. The three `TODO` rows are removed. The strategic context now cites `docs/VISION.md`. The scope of PB-1 gains the task queue, the pick-up procedure, and issues #49 and #50. A new subsection, "Tasks", says that `docs/TASKS.md` holds the tasks and that only the maintainer changes the status of an item. MR-1 is unchanged.
2. `docs/assurance/DEVELOPMENT-FRAMEWORK.md` gains one section, "Pick up the next task". It tells an agent how to choose a task from `docs/TASKS.md`, claim it, run the existing chain, and record the result. No existing stage, gate, or workflow description changes.

### Rationale

The maintainer asked on 2026-09-30 for progress to be tracked in the repository, so that "pick up next task" is enough for an agent to continue work on the vision under the development framework. Two facts on `main` block that today. The Next, Medium-term and Aspirational tables of the roadmap hold only `TODO`, so no item governs any protected-surface change towards `docs/VISION.md`, and `/protected-surface-amend` refuses without one. The framework says how to make a change and does not say how to choose one. Intent: `intent/2026-09-30-task-queue.md`. Spec: `intent/2026-09-30-task-queue-spec.md`.

An agent drafted the nine roadmap items from `docs/VISION.md` and the open issues. Resolved on 2026-09-30 by harry-nicholls (maintainer): the maintainer reviewed the scope, acceptance and horizon of each item and approved them as they stand in this pull request, including AD-1 as reworded in `c62a7a9` (review each imported issue against the vision, then refine or drop it).

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (immediate horizon, item PB-1)
- **Title:** Adopt the AI-native SDLC playbook as the development framework and make every human gate self-explanatory
- **Scope coverage:** PB-1 owns the development framework and its documents. Its acceptance is that a contributor can trace a change from intent to merge with `DEVELOPMENT-FRAMEWORK.md` alone. Choosing the change is the step before stage 1 of that chain.

No item existed that covers adding roadmap items for the vision. This note cites PB-1 because PB-1 governs the framework documents. The note `restore-hooks-and-mr-1-2026-09-29.md` added MR-1 under MR-1 itself, so there is no exact precedent. Resolved on 2026-09-30 by harry-nicholls (maintainer): PB-1 may authorise the nine new roadmap items in this change, and no separate item for roadmap growth is needed first.

### Authority

- **Authoriser:** harry-nicholls. Asked for the outcome on 2026-09-30, then reviewed the roadmap items and the pick-up procedure and approved them the same day. The merge is the approval.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `docs/assurance/ROADMAP.md` | Strategic context | replaced: cites `docs/VISION.md` |
| 2 | `docs/assurance/ROADMAP.md` | Horizon index, Immediate | PB-1 scope extended; paragraph on `docs/TASKS.md` added |
| 3 | `docs/assurance/ROADMAP.md` | Horizon index, Next | `TODO` row replaced by VA-1, ER-1, CG-1 |
| 4 | `docs/assurance/ROADMAP.md` | Horizon index, Medium-term | `TODO` row replaced by TB-1, RQ-1, AD-1 |
| 5 | `docs/assurance/ROADMAP.md` | Horizon index, Aspirational | `TODO` row replaced by CE-1, SC-1, CL-1 |
| 6 | `docs/assurance/ROADMAP.md` | Horizon index, new subsection "Tasks" | added: tasks live in `docs/TASKS.md` |
| 7 | `docs/assurance/DEVELOPMENT-FRAMEWORK.md` | new section "Pick up the next task" | added |

### Test / Coverage Impact

- Both files are Class A. No invariant and no property test changes.
- The tier gate reports `docs/assurance/**` as "not yet reached: human review is the only evidence". No deterministic check exercises either file.
- No attestation or intent-check baseline depends on either file.
- `docs/TASKS.md` is new and is not a protected path. No deterministic check reads it yet. Task `PB-1.6` in the queue adds one.

### Review Checklist

- [x] Rationale is anchored to a concrete trigger (the maintainer's request of 2026-09-30).
- [x] Authoriser is a named human.
- [x] PB-1 may authorise new roadmap items, or a different item is cited.
- [x] Each of the nine new items has the scope, acceptance, and horizon the maintainer wants.
- [x] The order of `docs/TASKS.md` is the order the maintainer wants.
- [x] The pick-up procedure weakens no gate: the agent still writes an intent first, still declares a tier, and still stops for a person at an open question.
- [x] This amendment block appears in the PR body.
- [x] All `REQUIRES HUMAN VERIFICATION:` markers above have been resolved.
