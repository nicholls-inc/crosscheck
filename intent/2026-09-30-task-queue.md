# Intent: A task queue that any agent can pick up from

## Problem statement
The repository records how to make a change, and it does not record which change to make next.

- `docs/assurance/ROADMAP.md` has two items. MR-1 is done and PB-1 is in progress. The Next, Medium-term and Aspirational tables hold only `TODO`.
- `docs/VISION.md` was agreed on 2026-09-29. No roadmap item covers any of its seven design rules, so no protected-surface change towards the vision has an item to cite.
- The backlog is 35 open GitHub issues with no order between them.
- A new session starts with none of this in its context. The maintainer has to ask an agent to reconstruct the status before each piece of work.

## Proposed outcome
- The roadmap has an item for each part of the vision, in the horizon where the maintainer expects to reach it.
- `docs/TASKS.md` is an ordered queue of tasks. Each task is one pull request. Its ID names the roadmap item that governs it.
- `docs/assurance/DEVELOPMENT-FRAMEWORK.md` has a procedure that an agent follows when the maintainer says "pick up next task". The procedure chooses one task, claims it, and then runs the existing chain from stage 1.
- `CLAUDE.md` and `AGENTS.md` point at the procedure, so an agent in any runtime finds it without being told.
- The pull request that completes a task marks that task done. The maintainer's merge makes the new status true on `main`.

## Affected users and systems
- The maintainer, who no longer briefs each session on the status.
- Coding agents in any runtime. They read the queue and the procedure from the repository.
- `docs/assurance/ROADMAP.md` and `docs/assurance/DEVELOPMENT-FRAMEWORK.md`, which are protected surfaces.
- GitHub issues. They stay where discussion happens. A task links to its issue.

## Constraints
- An agent proposes roadmap items. It does not approve them. The maintainer's merge is the approval.
- A change of status must not force Tier 3. The queue therefore lives outside the protected paths, and it grants no authority. Only a roadmap item governs a protected-surface change.
- The procedure adds no step that an LLM judges. An agent chooses a task by rules that a script could apply.
- No skill, agent, MCP tool, hook, or CI job changes behaviour.
- No gate, tier, or protected path changes.

## Open questions
None block the spec. The scope, the acceptance, the horizon, and the order of the proposed roadmap items are the maintainer's to decide. The pull request presents them as a proposal, and the governance note marks them `REQUIRES HUMAN VERIFICATION`.
