# Assurance Roadmap

## Purpose

This directory tracks the execution of the 6-layer assurance hierarchy as
applied pragmatically to this repository. It exists so the plan survives across
long timeframes without depending on conversation context or ephemeral plan
files. Each numbered item below is the source of truth for its own scope,
acceptance criteria, and kill criteria; larger items get a standalone doc under
`immediate/`, `next/`, `medium-term/`, or `aspirational/`.

This roadmap is deliberately minimal. Crosscheck dogfoods the tooling it ships,
so the first item is the framework by which every later item will be built.

## Strategic context

The long-term goal is stated in [`../VISION.md`](../VISION.md), agreed on
2026-09-29: every change an AI makes ships with a record of evidence, each claim
in the record names its strength, and no guarantee rests on the judgement of an
LLM. The items below are the steps towards that vision. Each item names the
design rule of the vision that it serves.

Before the vision, the goal was stated as formally verified kernels for critical
pure logic, contract graphs for integration boundaries, and spec-intent
alignment checks for everything else. Crosscheck and CGV implement parts of
that. PB-1 gave the repository a development framework that applies the tools to
itself, and human gates a newcomer can act on without asking anyone.

Current projection across the six layers: `TODO: fill from
/assurance-layer-audit output`.

## Dual-track enforcement principle

> Every deterministic assurance check added by this roadmap must produce two
> enforcement points:
>
> 1. **Pre-commit hook** — cheap, fast (< 5 s), lightweight. Blocks the commit
>    locally. Must emit a human-readable error that includes the exact command
>    to resolve the failure. LLMs acting as coding agents will see this error
>    and must be able to fix it by following the instruction without human
>    intervention.
> 2. **CI job** — slower, more comprehensive, runs on every PR regardless of
>    how the code change was authored. Must also emit actionable fix
>    instructions. Can perform work too expensive for pre-commit (full test
>    suites, container-based verification).
>
> Workflow phases are **not a substitute** for either enforcement point.
>
> Pre-commit hooks are fast attestation checks only — they must never invoke
> LLMs or run slow test suites. Heavy verification lives in CI and in dedicated
> binaries that the pre-commit hook verifies were run.

## Horizon index

### Immediate (start now)

| # | Item | Cost | Doc |
|---|---|---|---|
| PB-1 | Adopt the AI-native SDLC playbook as the development framework and make every human gate self-explanatory | M | [`DEVELOPMENT-FRAMEWORK.md`](DEVELOPMENT-FRAMEWORK.md) |
| MR-1 | Consolidate Crosscheck and the contract graph verifier into one repository under one vision | M | [`../VISION.md`](../VISION.md) |

**PB-1 — Status: In progress.** Scope: adopt the playbook artefact chain
(intent → spec → plan → diff + tests → PR with review findings → incident
record + eval) as the development framework for both Crosscheck and CGV. It
includes a tier map, a protected-surface hook, `REVIEW.md`, and deterministic CI:
- `tier-gate.yml`, with the gate's own tests;
- `ci.yml` for Crosscheck;
- `cgv-ci.yml` for CGV, which includes the theorem-statement manifest check;
- `incident-eval-check.yml`.

No CI job calls an LLM, so only deterministic checks and the maintainer's merge
count as evidence. PB-1 also gives every human gate a plain-language explainer
under `docs/gates/`, reached from a fixed three-sentence gate message.
Acceptance: a new contributor can trace a change from `intent/<slug>.md` to a
merged PR using `DEVELOPMENT-FRAMEWORK.md` alone, and every gate in the
inventory names its explainer. Later artefacts cite PB-1 as their governing
roadmap item. The move to CI with deterministic evidence only is recorded in
`intent/2026-09-29-deterministic-evidence.md`. PB-1 also covers the task queue
in `docs/TASKS.md` and the procedure "Pick up the next task" in
`DEVELOPMENT-FRAMEWORK.md`, recorded in `intent/2026-09-30-task-queue.md`.
Issues: #49, #50.

**MR-1 — Status: Done** (#18, #42, and nicholls-inc/claude-code-marketplace#251).
Scope: move the Crosscheck plugin, its development framework, its CI, its
release tags, and its open issues from `nicholls-inc/claude-code-marketplace`
into this repository with history, next to the contract graph verifier in
`cgv/`, and install the plugin from here with a `git-subdir` marketplace
source. Acceptance: `claude plugin install crosscheck@nicholls` installs the
plugin from `crosscheck/` of this repository, and both tools' protected surfaces
are listed in `.claude/rules/protected-surfaces.md`. Intent:
`intent/2026-09-29-crosscheck-monorepo.md`.

### Next (4–8 weeks)

| # | Item | Cost | Doc |
|---|---|---|---|
| VA-1 | Bring what Crosscheck tells its users in line with the vision | M | [`../VISION.md`](../VISION.md) |
| ER-1 | Define the evidence record, and emit it from both tools | L | [`../VISION.md`](../VISION.md) |
| CG-1 | Make CGV findings worth reading on a real codebase | M | [`../../cgv/docs/evaluation/real-codebase-evaluation-2026-09.md`](../../cgv/docs/evaluation/real-codebase-evaluation-2026-09.md) |

**VA-1 — Status: Not started.** Serves rule 1 and the scope section of the
vision. Scope: the skills, the agents, `crosscheck/README.md` and
`crosscheck/docs/assurance-hierarchy.md` still describe the positions that the
vision replaces. They tell a target repository that Tier 3 needs an
`intent-check` attestation, they call spec completeness "best-effort", and they
call some classes of code "out of scope" or "not addressed". Acceptance: no
skill or agent presents an LLM verdict as evidence or as a required artefact,
and each class of code that a tool does not reach is described as "not yet
reached", with the property that blocks it and the open question.

**ER-1 — Status: Not started.** Serves rules 3 and 7, and the vision's central
claim. Scope: one documented format for the record of evidence that ships with a
change. Each claim names its strength (proved, tested, observed, or judged), its
trusted base, and the command that reruns it. Acceptance: CGV and one Crosscheck
pipeline each emit a record in the format, and a deterministic checker rejects a
record with a claim that names no strength or no rerun command. How evidence of
different strengths combines into one verdict is an open question of the vision,
and the intent for ER-1 must flag it.

**CG-1 — Status: Not started.** Serves rule 2. Scope: on the one real codebase
measured so far, 3 of 67 triaged errors were reachable bugs. The work is fewer
false errors, fewer real bugs reported as warnings, a baseline mode, a checkable
witness for each error, and a README that says what exit 0 means. Acceptance:
the issues below are closed, and `cgv/bench` reports precision on a labelled
corpus for each release. Issues: #5, #6, #7, #8, #9, #10.

### Medium-term (2–3 months)

| # | Item | Cost | Doc |
|---|---|---|---|
| TB-1 | Shrink the trusted base, and measure what remains | L | [`../../cgv/README.md`](../../cgv/README.md) (Trust model) |
| RQ-1 | Write requirements formally, and prove that a spec achieves them | L | [`../VISION.md`](../VISION.md) |
| AD-1 | Work through the Crosscheck backlog imported from the marketplace repository | L | #27 |

**TB-1 — Status: Not started.** Serves rule 2. Scope: CGV's extraction from
Python is not proved, a declaration can skip the Lean kernel, and no checker has
a published record of the seeded errors it rejected. Acceptance: each issue
below is closed or has a recorded decision, and each checker in the suite
publishes the seeded errors that it rejected and the ones it missed. A second,
independently written proof checker is not yet reached. The property that
blocks it is an export of the proofs in a form a second checker reads, and the
open question is which checker to use. Issues: #16, #47, #48, #51.

**RQ-1 — Status: Not started.** Serves the second link of the vision's chain:
the spec and the assumptions give the requirement. Scope: a formal statement of
a requirement, a proof that a spec and its stated assumptions achieve it, and a
trace from a CGV contract to the requirement it serves. Acceptance: one worked
example in `formal-verification/` carries a requirement, a spec, stated
assumptions, and a machine-checked proof that links them.

**AD-1 — Status: Not started.** Scope: the 23 issues that came with the import
of Crosscheck. They cover the gap between the ADD design and what ships (#27),
the conformance oracle, the orchestrator, and four field reports. Acceptance:
each issue is closed, or is a task in `docs/TASKS.md` under the item it belongs
to. Issues: #19 to #41.

### Aspirational (scope and commit later)

| # | Item | Cost | Doc |
|---|---|---|---|
| CE-1 | Write specs in a controlled English, with a glossary of the organisation's terms | L | [`../VISION.md`](../VISION.md) |
| SC-1 | Generate scenarios that a spec allows and forbids, for a domain expert to judge | L | [`../VISION.md`](../VISION.md) |
| CL-1 | Reach more classes of code | L | [`../VISION.md`](../VISION.md) |

**CE-1 — Status: Not started.** Serves rules 4 and 5. Two open questions of the
vision come first: which controlled English to use, and who owns the glossary.

**SC-1 — Status: Not started.** Serves rule 6. Scope: a solver produces the
scenarios. `/spec-adversary` uses an LLM, so under rule 1 it is a search tool
and its output is not evidence.

**CL-1 — Status: Not started.** Serves the scope section of the vision. Each
class in the vision's table becomes its own item when work on it starts.

### Tasks

`docs/TASKS.md` holds the ordered queue of tasks. One task is one pull request,
and the ID of a task names the item above that governs it. An agent that is
told "pick up next task" follows "Pick up the next task" in
[`DEVELOPMENT-FRAMEWORK.md`](DEVELOPMENT-FRAMEWORK.md). The queue grants no
authority: a change to a protected surface cites an item in this file, never a
task. An agent marks a task done in the pull request that completes it. Only
the maintainer changes the `Status:` of an item.

## Kill criteria

Stop and re-plan the entire roadmap if any of the following becomes true.

- The formal-verification pipeline remains unusable for a first kernel after a
  sustained attempt, so no verified kernel is reachable.
- The spec-alignment false-positive rate breaches the 30% ceiling enforced by
  `/intent-check`, meaning the alignment signal costs more than it earns.
- No immediate-horizon item merges within four weeks, meaning the roadmap is
  aspirational in practice.

## How to use this directory

Every item carries a `Status:` field: `Not started`, `In progress`, `Blocked`
(with a `Blocker:` line), `Done` (cross-referencing the landing PR), or
`Deferred` (with a required `Reason:` line and a link to any superseding item).
Items are never deleted — only marked `Deferred`. IDs are stable: once an item
has an ID, that ID is never reused or renumbered, because other artefacts cite
it as their governing roadmap item.

## References

- [`DEVELOPMENT-FRAMEWORK.md`](DEVELOPMENT-FRAMEWORK.md) — the artefact chain
  and its triggers.
- [`TIER-LAYER-MAP.md`](TIER-LAYER-MAP.md) — change tiers and required
  artefacts.
- `../../REVIEW.md` — review passes and finding severities.
