## Protected-Surface Amendment

**Target file(s):** `scripts/ci/task-queue.mjs`, `scripts/ci/task-queue.test.mjs`, `.github/workflows/task-queue.yml`, `docs/assurance/DEVELOPMENT-FRAMEWORK.md`
**Class:** A (CI enforcement, governance documents)
**Matched rule:** `scripts/ci/**`, `.github/workflows/**`, `docs/assurance/**`
**Date:** 2026-10-01

### Change Description

1. `scripts/ci/task-queue.mjs` (new): `next` prints the next task by the three conditions of step 1 of "Pick up the next task" (NX-1 to NX-4). `check` fails when a row's task ID names no roadmap item, a task ID repeats, a status is not `todo`, `blocked` or `done`, a dependency names no row, or the pull request sets to `done` a row other than the one its `Task:` line names (QC-1 to QC-6). It exits 2 when a queue table has no separator row (QP-4), rather than dropping its first row, and when the base file exists but has no queue (QC-7), rather than treating it as missing.
2. `scripts/ci/task-queue.test.mjs` (new): TT-1 to TT-4, including exit 2 for QP-4 in the working tree and at the base, and for QC-7. TT-3 runs the claim snippet from `DEVELOPMENT-FRAMEWORK.md` twice at once against a scratch bare remote and asserts exactly one winner.
3. `.github/workflows/task-queue.yml` (new): runs `check` on every pull request, with the base ref and the PR body passed as environment variables (WF-1).
4. `docs/assurance/DEVELOPMENT-FRAMEWORK.md`: step 1 of the pick-up procedure names `node scripts/ci/task-queue.mjs next` (DOC-1). Stage 5 lists `task-queue.yml` (DOC-2). The rules of the procedure do not change.

### Rationale

Task PB-1.6 in `docs/TASKS.md`. The queue's own spec flags that nothing checks the queue and that a `done` is self-attested, and names PB-1.6 as the fix (`intent/2026-09-30-task-queue-spec.md`, "Concerns flagged, not resolved here"). Intent: `intent/2026-10-01-queue-check.md`. Spec: `intent/2026-10-01-queue-check-spec.md`. Plan: `intent/2026-10-01-queue-check-plan.md`.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (immediate horizon, item PB-1)
- **Title:** Adopt the AI-native SDLC playbook as the development framework and make every human gate self-explanatory
- **Scope coverage:** PB-1's scope covers the task queue in `docs/TASKS.md` and the procedure "Pick up the next task", and deterministic CI for the framework. Task PB-1.6 is this change.

### Authority

- **Authoriser:** harry-nicholls. The merge is the approval.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `scripts/ci/task-queue.mjs` | whole file | added |
| 2 | `scripts/ci/task-queue.test.mjs` | whole file | added |
| 3 | `.github/workflows/task-queue.yml` | whole file | added |
| 4 | `docs/assurance/DEVELOPMENT-FRAMEWORK.md` | "5. Deploy", "Pick up the next task" step 1 | reworded |

### Test / Coverage Impact

- `node --test scripts/ci/*.test.mjs`, run by the Tier Gate job, gains TT-1 to TT-4.
- A new check, `Task Queue`, reports on every pull request. The ruleset requires no status checks, so it cannot block a merge.
- No gate, tier, protected path, invariant or eval changes. The tier gate's result does not change.

### Review Checklist

- [x] Rationale is anchored to task PB-1.6 and the flagged concerns of the queue spec.
- [x] Authoriser is a named human.
- [x] PB-1 covers the task queue and the pick-up procedure.
- [x] The rules of the pick-up procedure are unchanged; the script applies them.
- [x] No `run:` line in the new workflow contains a `${{ }}` expression.
- [x] A queue the script cannot read exits 2 with the file named (QP-4, QC-7), and never passes or counts rows it could not read.
