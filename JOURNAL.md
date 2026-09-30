# JOURNAL.md

This is the repo-root journal — the broadest shard in the sharded-journal architecture described in [ADR-0001](docs/decisions/0001-sharded-journal-architecture.md). It records decisions that cut across the whole repo (plugins, marketplace conventions, tooling). Other shards live further down the tree at meaningful design boundaries and carry narrower decisions. Entries are newest first. Before non-trivial work in any directory, walk up reading every `JOURNAL.md` you pass — see [AGENTS.md](AGENTS.md) for the rule.

---

## 2026-09-30 - The tier gate reads `Tier:` only at the start of a line

**Type:** fix
**Touches:** scripts/ci/tier-gate.mjs, scripts/ci/tier-gate.test.mjs, docs/assurance/TIER-LAYER-MAP.md, docs/gates/tier-layer-gate.md, intent/2026-09-29-deterministic-evidence-spec.md, docs/TASKS.md
**Why:** Pasted text such as "Tier: 1" could declare a tier, and the pass report called unchecked code "none required at this tier" (#50).
**Links:** [intent](intent/2026-09-30-tier-anchor.md), [spec](intent/2026-09-30-tier-anchor-spec.md), [plan](intent/2026-09-30-tier-anchor-plan.md)

A `Tier:` line now follows the same rule as a citation: it starts the line, optionally after a list or quote marker. The pass report's classes are now rows of three kinds. A checked row names its workflow. A not-yet-reached row names the blocking property and the open question. Only prose is "none required at this tier". Two paths were misreported and now name their checks: `crosscheck/conformance/**` (the conformance job) and the protected-surface hook (its tests in the Tier Gate job).

---

## 2026-09-30 — Merged governance notes no longer unlock the hook

**Type:** fix
**Touches:** .claude/hooks/protected-surface-guard.mjs, scripts/ci/protected-surface-guard.test.mjs, .claude/rules/protected-surfaces.md, docs/assurance/DEVELOPMENT-FRAMEWORK.md, docs/gates/protected-surface-hook.md, crosscheck/skills/assurance-init/SKILL.md, docs/TASKS.md, CLAUDE.md
**Why:** On main at 608ca86 the hook allowed edits to 30 of 58 protected files with no new note because it counted notes that had already merged to the default branch.
**Links:** [intent](intent/2026-09-30-merged-notes-unlock.md), [spec](intent/2026-09-30-merged-notes-unlock-spec.md), [plan](intent/2026-09-30-merged-notes-unlock-plan.md)

The hook now compares a governance note's text on this branch against the default branch (origin/HEAD else origin/main). Only notes that are new on this branch count; notes already merged to main no longer unlock edits. This mirrors the Tier Gate's rule that a note from an earlier change does not count.

---

## 2026-09-30 — A task queue beside the roadmap

**Type:** intent-refinement
**Touches:** docs/TASKS.md (new), docs/assurance/ROADMAP.md, docs/assurance/DEVELOPMENT-FRAMEWORK.md, CLAUDE.md, AGENTS.md
**Why:** Each new session had to be told where the work stood, and the roadmap had no item for any part of the vision.
**Links:** [intent](intent/2026-09-30-task-queue.md), [spec](intent/2026-09-30-task-queue-spec.md), [queue](docs/TASKS.md)

The roadmap now has an item for each part of the vision, and `docs/TASKS.md` is the ordered queue of tasks under those items. The two are separate files on purpose. The roadmap is a protected surface, so a status change there is a Tier 3 change with a governance note. The queue is not protected, so the pull request that finishes a task can mark it done at whatever tier the task has. The queue has no "in progress" status, because a status on a branch is invisible to every other branch. A pushed branch named `task/<task ID>` is the claim. It holds one empty commit with a unique message and is pushed with a lease that fails if the branch exists, because a plain push of a branch cut from `origin/main` succeeds a second time with "Everything up-to-date". Nothing deterministic checks the queue yet, and task PB-1.6 adds that check.

## 2026-05-11 — Sharded journals plus a root walk-up rule [ADR-0001]

**Type:** intent-refinement
**Touches:** AGENTS.md (new), JOURNAL.md shards (new at repo root, crosscheck/, crosscheck/docs/add/), docs/decisions/ (new)
**Why:** We wanted a shared narrative record that humans and agents both read by default, and the first try inside the Crosscheck plugin was archived after one shipped iteration.
**Links:** [ADR-0001](docs/decisions/0001-sharded-journal-architecture.md), [v2 retrospective](crosscheck/docs/add/.retrospective/findings-and-methodology-v2.md)

This repo is starting to use co-located `JOURNAL.md` files plus a root `AGENTS.md` walk-up rule. The first place it lands is the Crosscheck plugin's design work, which is where the need surfaced. The shape is small on purpose — a header per file, one entry per decision, plain product voice, frontmatter for type and links. It may change once it gets driven against real spec sessions; the retrospective is candid that nothing about the working hypothesis is settled yet.
