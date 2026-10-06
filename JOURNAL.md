# JOURNAL.md

This is the repo-root journal — the broadest shard in the sharded-journal architecture described in [ADR-0001](docs/decisions/0001-sharded-journal-architecture.md). It records decisions that cut across the whole repo (plugins, marketplace conventions, tooling). Other shards live further down the tree at meaningful design boundaries and carry narrower decisions. Entries are newest first. Before non-trivial work in any directory, walk up reading every `JOURNAL.md` you pass — see [AGENTS.md](AGENTS.md) for the rule.

---

## 2026-10-06 - The intent-check attestation is an advisory record

**Type:** fix
**Touches:** crosscheck/skills/intent-check/SKILL.md, crosscheck/skills/intent-check/references/attestation-schema.md, crosscheck/skills/assurance-init/SKILL.md, crosscheck/skills/protected-surface-amend/SKILL.md, crosscheck/skills/draft-invariants/SKILL.md, crosscheck/agents/hellebuyck.md, crosscheck/agents/add-orchestrator.md, crosscheck/agents/lowry.md, crosscheck/docs/orchestrator-coordination.md, docs/gates/intent-check-verdict.md, docs/gates/intent-check-kill-criterion.md, docs/gates/README.md, docs/TASKS.md
**Why:** This repository stopped counting the attestation as a Tier 3 artefact on 2026-09-29, but the skills and agents it ships still told other repositories to gate commits on an LLM `pass`, to list the attestation as a Tier 3 artefact, and to accept it as amendment authority.
**Links:** [intent](intent/2026-10-06-intent-check-advisory.md), [spec](intent/2026-10-06-intent-check-advisory-spec.md), [plan](intent/2026-10-06-intent-check-advisory-plan.md)

`/intent-check` still runs the round trip, appends the tracker row and writes `.assurance/intent-check-attestation.json`, with the same schema and hash. It no longer drafts a pre-commit hook that rejects a commit without a passing attestation, and it tells the user to remove one that an earlier version drafted. A failed verdict now offers a fourth route, classifying the verdict as spurious. `add-orchestrator` and `/draft-invariants` cite the hash algorithm by section heading, since removing the hook sections moved the lines they cited. No check enforces the new wording. The example workflows under `crosscheck/docs/examples/workflows/` still describe a mandatory intent-check gate, and task VA-1.3 covers them.

---

## 2026-10-06 - A pre-commit hook runs the checks that need no PR body

**Type:** feature
**Touches:** .husky/pre-commit, scripts/ci/pre-commit.mjs, scripts/ci/pre-commit.test.mjs, scripts/ci/tier-gate.mjs, scripts/ci/tier-gate.test.mjs, scripts/ci/task-queue.mjs, docs/assurance/DEVELOPMENT-FRAMEWORK.md, docs/assurance/TIER-LAYER-MAP.md, docs/gates/tier-layer-gate.md, docs/gates/task-queue-check.md, docs/TASKS.md
**Why:** The roadmap's dual-track principle asks every deterministic check for a pre-commit hook as well as a CI job. The tier gate and the task queue check had only the CI job, and the PreToolUse hook sees only Claude Code's edit tools.
**Links:** [intent](intent/2026-10-06-pre-commit-hooks.md), [spec](intent/2026-10-06-pre-commit-hooks-spec.md), [plan](intent/2026-10-06-pre-commit-hooks-plan.md)

`.husky/pre-commit` runs `node scripts/ci/pre-commit.mjs` on the commit as staged. A commit that stages a protected path fails unless every protected path the branch changes is named in a governance note the branch changes, which is the tier gate's TG-5 with the branch set taken from the index against the merge base. A commit that stages the queue or the roadmap fails on QC-1 to QC-4. The `Task:` line, the tier declaration, the citations and the CGV section live in the PR body, so they stay in CI. The hook reuses the CI scripts' own functions, so the two enforcement points cannot drift apart on the rules they share. It never fetches, so a stale `origin/main` can make it disagree with CI, and `--no-verify` skips it. On this repository a failing commit took 0.26 s.

---

## 2026-10-06 - The Tier Gate reads changed file names NUL-separated

**Type:** fix
**Touches:** .github/workflows/tier-gate.yml, scripts/ci/tier-gate.mjs, scripts/ci/tier-gate-workflow.test.mjs, intent/2026-09-29-deterministic-evidence-spec.md, docs/TASKS.md
**Why:** `git diff --name-only` C-quotes a name with a non-ASCII byte, a double quote, a backslash or a control character. The quoted name matched no protected glob, so `docs/assurance/é.md` passed the gate at `Tier: 1`.
**Links:** [intent](intent/2026-10-06-unquoted-paths.md), [spec](intent/2026-10-06-unquoted-paths-spec.md), [plan](intent/2026-10-06-unquoted-paths-plan.md)

The step now writes `git diff -z` to a temporary file and passes its path as `CHANGED_FILES_PATH`. The gate splits on NUL and keeps each name as written. `core.quotePath=false` was not enough, since it still quotes `"`, `\` and control characters. Once a name with a newline reached the gate intact, `**` compiled to `.*` still stopped at the newline, so each glob now compiles with the `s` flag. `.husky/commit-msg` has the quoting fault (PB-1.13), and the protected-surface hook has the newline fault (PB-1.14). The gate still reads an unset `CHANGED_FILES_PATH` as an empty list, which fails open. The maintainer kept that out of this task, and PB-1.15 makes it fail closed.

---

## 2026-10-06 - The tier gate's explainer stops claiming it checks incident evals

**Type:** docs
**Touches:** docs/gates/tier-layer-gate.md, docs/TASKS.md
**Why:** The explainer said the tier gate expects an incident's eval before it passes. The tier gate reads no incident reference. A run with the `incident` label, an incident id and no eval passes it.
**Links:** [intent](intent/2026-10-06-tier-gate-incident-doc.md)

The section now says that the Incident Eval Check, a separate workflow, checks incidents, names its trigger and the eval and candidate invariant it needs, and says that it runs after the merge and cannot block it. The run for #62 failed after its merge because the pull request quoted the trigger in prose. Task PB-1.16 covers that.

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
