# Intent: Only deterministic checks count as evidence in CI

## Problem statement
The vision says no guarantee rests on the judgement of an LLM. The repository's own gates break that rule, and they check less than they claim:

- Tier 3 requires an `intent-check` attestation, which records an LLM verdict. The tier gate also accepts any attestation file anywhere in the tree. The committed attestation from #134 therefore satisfies every Tier 3 pull request, and so does the root `plan.md` from #246.
- `protected-surface-check.yml` and `spec-audit.yml` run an LLM in CI, and there is no budget for that. Without an `ANTHROPIC_API_KEY` secret, `protected-surface-check.yml` fails on every pull request that touches a protected file.
- CGV has no CI. `cargo test`, `lake build`, and the fixture checks never run on a pull request.
- CGV's soundness theorem statements are protected by rule. Nothing deterministic notices when one of them changes.

## Proposed outcome
- Every check in CI is deterministic, and no workflow calls an LLM.
- The tier gate counts an artefact only if the pull request adds or changes it, or its author cites it by path.
- CGV has CI.
- A change to a protected theorem statement, or to a definition it relies on, fails CI unless a committed statement manifest changes with it. The manifest is a protected path, so that change forces Tier 3.
- The development framework covers CGV. Decisions are recorded in `intent/2026-09-29-crosscheck-monorepo.md`.

## Affected users and systems
- Contributors to both tools. A Tier 3 pull request no longer needs an attestation. It must carry its own plan, or cite one, together with its own governance note.
- CI: `tier-gate.yml` gains tests, a new `cgv-ci.yml` is added, and `spec-audit.yml` and `protected-surface-check.yml` are removed.
- The framework documents, which say what counts as evidence: `TIER-LAYER-MAP.md`, `DEVELOPMENT-FRAMEWORK.md`, `protected-surfaces.md`, and `docs/gates/`.

## Constraints
- No LLM runs in CI, because there is no budget for it.
- There is no branch protection, because the repository is private and the account has no GitHub Pro. No check can block a merge, and the maintainer's merge is the human sign-off.
- No Crosscheck skill, agent, or MCP tool changes behaviour in this change. `/intent-check` and the audit skills stay as they are. They run locally, as advisory tools.
- No CGV theorem, definition, or proof changes. The manifest records the current statements.

## Open questions
None. The maintainer decided each question on 2026-09-29.
