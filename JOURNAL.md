# JOURNAL.md

This is the repo-root journal — the broadest shard in the sharded-journal architecture described in [ADR-0001](docs/decisions/0001-sharded-journal-architecture.md). It records decisions that cut across the whole repo (plugins, marketplace conventions, tooling). Other shards live further down the tree at meaningful design boundaries and carry narrower decisions. Entries are newest first. Before non-trivial work in any directory, walk up reading every `JOURNAL.md` you pass — see [AGENTS.md](AGENTS.md) for the rule.

---

## 2026-10-06 - The framework states what the Incident Eval Check does

**Type:** docs
**Touches:** docs/assurance/DEVELOPMENT-FRAMEWORK.md, docs/TASKS.md
**Why:** Stage 5 said the check fails on an incident record under `evals/` with no eval. The check never looks for incident records, needs a candidate invariant too, has an exit 2, and runs only after the merge.
**Links:** [intent](intent/2026-10-06-incident-eval-doc.md), [plan](intent/2026-10-06-incident-eval-doc-plan.md)

The bullet now names the trigger (the `incident` label, or a `Fixes-Incident:` line in the body or a commit), the eval and the candidate invariant it needs, exit 1, and exit 2 for commits it cannot read. It also says that the workflow runs on a merged pull request, so it reports on a merge and cannot block one. The run for #61 started four seconds after the merge, and the run for #60, closed without a merge, was skipped. `docs/gates/tier-layer-gate.md` still says the tier gate expects the eval before it passes. Task PB-1.11 fixes that.

---

## 2026-10-01 - The Tier Gate step computes its own changed files

**Type:** fix
**Touches:** .github/workflows/tier-gate.yml, scripts/ci/tier-gate-workflow.test.mjs, intent/2026-09-29-deterministic-evidence-spec.md, docs/TASKS.md
**Why:** A changed file named `EOF` ended the workflow's `$GITHUB_OUTPUT` block early. With a second file named `docs/assurance/a<<EOF`, scratch pull request #60 changed a protected file at `Tier: 1` and the gate passed on an empty list.
**Links:** [intent](intent/2026-10-01-tier-gate-workflow.md), [spec](intent/2026-10-01-tier-gate-workflow-spec.md), [plan](intent/2026-10-01-tier-gate-workflow-plan.md)

The gate step now fetches the base, sets `CHANGED_FILES` from `git diff` and runs the gate, so no file name passes through `$GITHUB_OUTPUT`. The base ref reaches the script as `BASE_REF` in `env:`, not as a `${{ }}` expression in shell source, and the gate's pass line now names it. A new test runs the step's script from the workflow file in a scratch repository, which also tests `--no-renames` for the first time. The test runs under local bash, not on a GitHub runner, so the runner evidence is #60 and this change's own Tier Gate run. `git diff --name-only` still quotes a path with a non-ASCII byte, which then matches no protected glob. Task PB-1.10 fixes that.

---

## 2026-10-01 - A script picks the next task, and CI checks the queue

**Type:** feature
**Touches:** scripts/ci/task-queue.mjs, scripts/ci/task-queue.test.mjs, .github/workflows/task-queue.yml, docs/assurance/DEVELOPMENT-FRAMEWORK.md, docs/gates/task-queue-check.md, docs/gates/README.md, docs/TASKS.md
**Why:** Agents read the queue by eye, nothing checked it, and a pull request could mark any row `done`. The queue's own spec named PB-1.6 as the fix.
**Links:** [intent](intent/2026-10-01-queue-check.md), [spec](intent/2026-10-01-queue-check-spec.md), [plan](intent/2026-10-01-queue-check-plan.md)

`node scripts/ci/task-queue.mjs next` applies the three conditions of the pick-up procedure, so two agents can no longer read the table two ways. `check` runs on every pull request. It fails on a task ID with no roadmap item, a repeated ID, an unknown status, or a missing dependency, and when a pull request sets to `done` a row its `Task:` line does not name. The `Task:` line uses the tier gate's anchor, so quoted text cannot name a task. The claim snippet stays in the framework document, and the race test runs that snippet itself rather than a copy, so an edit that breaks the claim breaks the test. The check does not require the completing pull request to set its own row, because the split rule lets a pull request replace a row and set nothing to `done`.

---

## 2026-09-30 - Any citation line counts, and only a regular file in the repository

**Type:** fix
**Touches:** scripts/ci/tier-gate.mjs, scripts/ci/tier-gate.test.mjs, docs/assurance/TIER-LAYER-MAP.md, docs/gates/tier-layer-gate.md, intent/2026-09-29-deterministic-evidence-spec.md, docs/TASKS.md
**Why:** Only the first `Plan:` line counted, and `existsSync` accepted a directory, `../x`, or a symlink out of the repository (#49).
**Links:** [intent](intent/2026-09-30-citation-rule.md), [spec](intent/2026-09-30-citation-rule-spec.md), [plan](intent/2026-09-30-citation-rule-plan.md)

The gate now reads every `Intent:`, `Spec:` or `Plan:` line, and one valid line meets the requirement. A cited path must resolve, through symlinks, to a regular file whose real path is inside the repository. The line format is unchanged. The gate still checks only that the file exists, not what it says, so any file in the repository satisfies a citation. That gap is recorded in TG-12 and is not new.

---

## 2026-09-30 - The tier gate reads `Tier:` only at the start of a line

**Type:** fix
**Touches:** scripts/ci/tier-gate.mjs, scripts/ci/tier-gate.test.mjs, docs/assurance/TIER-LAYER-MAP.md, docs/gates/tier-layer-gate.md, intent/2026-09-29-deterministic-evidence-spec.md, docs/TASKS.md
**Why:** Pasted text such as "Tier: 1" could declare a tier, and the pass report called unchecked code "none required at this tier" (#50). The maintainer's review decisions on #57 set the stricter rules.
**Links:** [intent](intent/2026-09-30-tier-anchor.md), [spec](intent/2026-09-30-tier-anchor-spec.md), [plan](intent/2026-09-30-tier-anchor-plan.md)

A `Tier:` line now starts the line, after an optional indent and no list or quote marker. The first `Tier:` line is the declaration, and an invalid one fails. A `Tier:` line and a `tier:N` label must agree: the map always said so, and the gate now enforces it. The pass report's classes are now rows of two kinds. A checked row names its workflow. A not-yet-reached row names the blocking property and the open question. Prose, `CLAUDE.md`, `AGENTS.md`, `REVIEW.md` and root `docs/invariants/**` are not yet reached too, so no line says "none required at this tier". The citations keep their list and quote markers, because a citation only names an existing file and cannot pick the tier. Two paths were misreported and now name their checks: `crosscheck/conformance/**` (the conformance job) and the protected-surface hook (its tests in the Tier Gate job).

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
