# JOURNAL.md

This is the repo-root journal — the broadest shard in the sharded-journal architecture described in [ADR-0001](docs/decisions/0001-sharded-journal-architecture.md). It records decisions that cut across the whole repo (plugins, marketplace conventions, tooling). Other shards live further down the tree at meaningful design boundaries and carry narrower decisions. Entries are newest first. Before non-trivial work in any directory, walk up reading every `JOURNAL.md` you pass — see [AGENTS.md](AGENTS.md) for the rule.

---

## 2026-09-30 — A task queue beside the roadmap

**Type:** intent-refinement
**Touches:** docs/TASKS.md (new), docs/assurance/ROADMAP.md, docs/assurance/DEVELOPMENT-FRAMEWORK.md, CLAUDE.md, AGENTS.md
**Why:** Each new session had to be told where the work stood, and the roadmap had no item for any part of the vision.
**Links:** [intent](intent/2026-09-30-task-queue.md), [spec](intent/2026-09-30-task-queue-spec.md), [queue](docs/TASKS.md)

The roadmap now has an item for each part of the vision, and `docs/TASKS.md` is the ordered queue of tasks under those items. The two are separate files on purpose. The roadmap is a protected surface, so a status change there is a Tier 3 change with a governance note. The queue is not protected, so the pull request that finishes a task can mark it done at whatever tier the task has. The queue has no "in progress" status, because a status on a branch is invisible to every other branch. A pushed branch named `task/<task ID>` is the claim. Nothing deterministic checks the queue yet, and task PB-1.6 adds that check.

## 2026-05-11 — Sharded journals plus a root walk-up rule [ADR-0001]

**Type:** intent-refinement
**Touches:** AGENTS.md (new), JOURNAL.md shards (new at repo root, crosscheck/, crosscheck/docs/add/), docs/decisions/ (new)
**Why:** We wanted a shared narrative record that humans and agents both read by default, and the first try inside the Crosscheck plugin was archived after one shipped iteration.
**Links:** [ADR-0001](docs/decisions/0001-sharded-journal-architecture.md), [v2 retrospective](crosscheck/docs/add/.retrospective/findings-and-methodology-v2.md)

This repo is starting to use co-located `JOURNAL.md` files plus a root `AGENTS.md` walk-up rule. The first place it lands is the Crosscheck plugin's design work, which is where the need surfaced. The shape is small on purpose — a header per file, one entry per decision, plain product voice, frontmatter for type and links. It may change once it gets driven against real spec sessions; the retrospective is candid that nothing about the working hypothesis is settled yet.
