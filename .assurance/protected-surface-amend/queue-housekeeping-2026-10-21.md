> **Action needed: Resolve markers and checklist before merging**
> You are being asked to verify this governance-note block as PR reviewer because the edit touches a protected surface. Approving means the protected-surface edit merges as governed and traceable; declining means the author must resolve every unresolved marker and Review Checklist item first. Full explanation: https://github.com/nicholls-inc/crosscheck/blob/main/docs/gates/protected-surface-amendment.md.

## Protected-Surface Amendment

**Target file(s):** `docs/assurance/ROADMAP.md`
**Class:** A (governance documents the gates read)
**Matched rule:** `docs/assurance/**`
**Date:** 2026-10-21

### Change Description

`docs/assurance/ROADMAP.md`: the `Status:` of VA-1, ER-1, CG-1, TB-1 and AD-1 changes from `Not started` to `In progress`, because each has at least one task that is `done` in `docs/TASKS.md` with a merged pull request. The RQ-1 paragraph gains `Task: RQ-1.1.` No scope, acceptance, kill criterion or other item changes. No item becomes `Done`.

### Rationale

Five items read `Not started` although tasks under them have merged, for example VA-1.1 (#72), ER-1.2 (#79), CG-1.1 (#75), TB-1.1 (#99) and AD-1.1 (#98). The roadmap says only the maintainer changes the `Status:` of an item. The maintainer told the coordinating agent, in chat on 2026-10-21, to do this housekeeping, including changing roadmap item statuses: "do the housekeeping. I give you explicit permission to merge PRs and ship the improvements and housekeeping tasks." Intent: `intent/2026-10-21-queue-housekeeping.md`. Spec: `intent/2026-10-21-queue-housekeeping-spec.md`.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (immediate horizon, item PB-1)
- **Title:** Adopt the AI-native SDLC playbook as the development framework and make every human gate self-explanatory
- **Scope coverage:** PB-1 covers the task queue in `docs/TASKS.md`, the pick-up procedure, and the rule that ties item statuses to the queue. Setting statuses from the queue keeps the two in step.

### Authority

- **Authoriser:** hnipps, who directed this change on 2026-10-21. The merge is the approval.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `docs/assurance/ROADMAP.md` | VA-1, ER-1, CG-1 (Next); TB-1, AD-1 (Medium-term) | `Status:` reworded: `Not started` to `In progress` |
| 2 | `docs/assurance/ROADMAP.md` | RQ-1 (Medium-term) | added: `Task: RQ-1.1.` |

### Test / Coverage Impact

- No invariant, property test or gate changes.
- `scripts/ci/task-queue.mjs` reads item IDs from `**<ID> — Status:` lines. Every ID still matches, and `node scripts/ci/task-queue.mjs check` passes on the branch.
- Class A: no eval or prompt-tuning follow-up. The roadmap is read by agents and people, not executed.

### Review Checklist

Mechanical items.

- [x] Authoriser is a named human, not a bot and not an agent.
  Command: the handle `hnipps` matches none of `*-bot`, `*[bot]*`, `claude-*` or an agent name. Output: no match.
- [x] Governing roadmap item exists.
  Command: `grep -n "PB-1 — Status" docs/assurance/ROADMAP.md`. Output: `62:**PB-1 — Status: In progress.** Scope: adopt the playbook artefact chain`.
- [x] Diff plan names every changed protected file.
  Command: `git diff --name-only origin/main...HEAD`, filtered by the globs in `.claude/rules/protected-surfaces.md`. Output: `docs/assurance/ROADMAP.md`.
- [x] This amendment block appears in the PR body.
  Command: the PR body contains the `## Protected-Surface Amendment` heading. Output: it does.
- [x] No unresolved human-verification marker is left.
  Command: count the markers in this file. Output: 0.

Judgment items. A human decides these, or an agent ticks them on the maintainer's explicit instruction.

- [ ] Rationale is anchored to a concrete trigger (not "cleanup" or "robustness").
- [ ] Governing roadmap item actually covers this change.
- [ ] Diff plan covers every affected line range and invariant ID / stage.
- [ ] No invariant is being weakened purely to make a failing test pass.
- [ ] ER-1 stays `In progress`, though each acceptance clause has a merged pull request, because ER-1.12 to ER-1.21 are open. The maintainer decides whether ER-1 is `Done`.
