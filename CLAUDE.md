# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working in this repository.

## What this is

A suite of tools for verifying AI-generated code. `docs/VISION.md` states what the suite is for.

| Path | What it holds | Guidance |
| --- | --- | --- |
| `crosscheck/` | The Crosscheck Claude Code plugin: skills, agents, and the MCP server | This file, and `crosscheck/README.md` |
| `cgv/` | The contract graph verifier: a Rust extractor and a Lean checker with soundness proofs | `cgv/CLAUDE.md` |
| `formal-verification/` | Specs and fixtures for the Lean and Dafny pipelines | |
| `docs/VISION.md` | The vision the whole suite shares | |

Run each tool's commands from its own directory. For example, run `cargo test` from `cgv/` and `npm test` from `crosscheck/mcp-server/`.

The plugin is published through the `nicholls` marketplace in `nicholls-inc/claude-code-marketplace`, which installs `crosscheck/` from this repository.

## Vision

Two rules from `docs/VISION.md` apply to every session:

- No class of code is out of scope. When a tool does not reach a class of code, say "not yet reached" and name the property that blocks it and the open research question. Never call a class "excluded" or "out of scope".
- No guarantee rests on the judgment of an LLM. An LLM may draft code, specs, and proofs, and LLM-based checks may point at likely problems, but only deterministic checks and human judgment count as evidence.

## Picking up work

`docs/TASKS.md` is the ordered queue of tasks, and `docs/assurance/ROADMAP.md` holds the items that govern them. When you are told "pick up next task", follow "Pick up the next task" in `docs/assurance/DEVELOPMENT-FRAMEWORK.md`.

## Crosscheck plugin (`crosscheck/`)

Crosschecks Claude's code claims using Dafny formal verification for provably correct Python/Go code, plus semi-formal reasoning for structured code analysis.

- **MCP server** (`crosscheck/mcp-server/`): TypeScript server exposing seven tools across two engines — Dafny (`dafny_verify`, `dafny_evidence` for an evidence record of a committed file, `dafny_compile`, `dafny_cleanup`) and Lean (`lean_check` for the `/lean-spec`, `/lean-impl`, `/correspondence-review`, and `/drt-oracle` build gates; `lean_run` for `/lean-impl` smoke checks and `/drt-oracle`'s per-def Lean runner; `lean_test` as a compile-time `#guard` path for fixture sanity checks)
- **Docker isolation**: Dafny 4.11.0 in a sandboxed container (no network, 512MB memory, 120s timeout); Lean 4 + Mathlib in a sister container with Mathlib oleans pre-warmed (no network, 2GB memory, 240s timeout)
- **Formal verification skills** (`crosscheck/skills/`): `/spec-iterate`, `/generate-verified`, `/extract-code`, `/lightweight-verify`
- **Lean executable-model + DRT-oracle pipeline** (`crosscheck/skills/`): `/informal-spec`, `/lean-spec`, `/lean-impl`, `/correspondence-review`, `/drt-oracle`
- **Spec management & adequacy skills** (`crosscheck/skills/`): `/check-regressions`, `/suggest-specs`, `/rationale`, `/audit-spec-coverage`, `/audit-invariant-consistency`
- **Semi-formal reasoning skills** (`crosscheck/skills/`): `/reason`, `/compare-patches`, `/locate-fault`, `/trace-execution`
- **Repository context skill** (`crosscheck/skills/`): `/journal-context` (deterministic walk of every `JOURNAL.md` from a path up to the repo root; load the narrative record before non-trivial design work)
- **Orchestrator agents** (`crosscheck/agents/`): `byfuglien` (implementation chain, sequential router), `hellebuyck` (specification chain, sequential router), `add-orchestrator` (ADD methodology workflow runner; parallel subagent dispatch + batched audit triage; drives spec → approved invariants ready for implementation)

### Development

```bash
cd crosscheck/mcp-server
npm install
npm run build            # Type-check + esbuild bundle → dist/index.js
npm test                 # Unit, integration, property, MCP tests (vitest)
npm run test:e2e         # End-to-end tests (requires Docker)
../scripts/build-docker.sh       # Build Dafny Docker image
../scripts/build-lean-docker.sh  # Build Lean+Mathlib Docker image (slow first time)
../scripts/test-mcp.sh           # Smoke tests
```

### Key conventions

- ES modules (type: "module" in package.json)
- Strict TypeScript (ES2022 target, Node16 module resolution)
- Zod for runtime validation of tool inputs
- Tests use vitest with fast-check for property-based testing
- Docker images configured via `DAFNY_DOCKER_IMAGE` (default `crosscheck-dafny:latest`) and `LEAN_DOCKER_IMAGE` (default `crosscheck-lean:latest`); Lean memory/cpu via `LEAN_DOCKER_MEMORY` / `LEAN_DOCKER_CPUS`

### Dafny limitations to keep in mind

- No IO/networking verification — requires `{:extern}` trust boundaries
- No concurrency modeling — sequential correctness only
- Go output uses type erasure to `interface{}` — may need type assertions
- `real` type compiles to `_dafny.BigRational`, not native floats

## Commit conventions

Conventional commits enforced via commitlint + husky. Use the tool as the scope: `crosscheck` or `cgv`.

**Behavioral artifacts** (`SKILL.md`, `agents/*.md`) define agent/skill behavior and are functional code. Commits that touch them must use a release-triggering type:

- `feat(crosscheck):` — new or expanded behavior (minor bump)
- `fix(crosscheck):` — corrective behavior change (patch bump)

`docs:` and `refactor:` are **both blocked** on behavioral artifacts (enforced by `.husky/commit-msg`). release-please treats `refactor:` as non-user-facing, so behavior changes filed as `refactor:` will silently stall the release pipeline — that is the failure mode behind the 2.4.0 → 2.5.0 backlog. If a change to `SKILL.md` or `agents/*.md` is genuinely non-behavioral (rare — usually internal renames or comment-only edits), split it into a separate commit that does not touch a behavioral artifact.

- `fix(crosscheck): correct abort threshold in /reason` — bug fix in skill logic
- `refactor(crosscheck): extract shared helper in mcp-server` — non-behavioral structural change outside `SKILL.md`/`agents/*.md`
- `docs(crosscheck): update README installation steps` — actual documentation (not a behavioral artifact)
- `fix(cgv): reject unknown constraint kinds in Translation.lean` — a CGV change

## Development framework

Every change to this repository starts as `intent/<slug>.md` — problem statement, proposed outcome, affected users and systems, constraints, open questions. Behavioural changes gain a committed `spec.md`, and anything touching a protected surface gains a `plan.md` too, before implementation begins. Do not open a PR whose stage artefacts do not exist.

- `docs/assurance/DEVELOPMENT-FRAMEWORK.md` — the artefact chain (intent → spec → plan → diff + tests → PR → incident record + eval + candidate invariant), which commit or event triggers each stage, and where each Crosscheck skill and agent sits in it.
- `docs/assurance/TIER-LAYER-MAP.md` — the three change tiers and the artefacts each one requires. PRs declare their tier in the body.
- `REVIEW.md` — the review passes (bugs and logic, security, compliance with spec and plan) and the Important/Nit severity rules.

## Protected surfaces

`.claude/rules/protected-surfaces.md` lists the protected surfaces of both tools. Crosscheck's surfaces (`SKILL.md`, `agents/*.md`, invariant docs, `docs/assurance/**`, `.claude/rules/**`, `.claude/hooks/**`, `evals/**`, CI) are guarded by a PreToolUse hook in `.claude/hooks/`, registered in `.claude/settings.json`: it blocks the edit unless a `/crosscheck:protected-surface-amend` governance note naming the file is new on this branch, not already on the default branch. CGV's surfaces are `cgv/prover/ContractGraph/BehaviorModel.lean` and the statements of its soundness theorems, which `cgv/prover/protected-statements.txt` records and CGV CI checks. No CI job calls an LLM. The default branch has a ruleset that asks for one approving review, but the maintainer is the only person who can approve and cannot approve their own pull request, so the maintainer merges by bypassing it: that merge is the human sign-off. The ruleset requires no status checks, so CI cannot block a merge.
