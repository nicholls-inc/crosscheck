# Intent: Skills and agents stop presenting the intent-check attestation as a required artefact

## Problem statement
`docs/VISION.md` rule 1 says no guarantee rests on the judgement of an LLM, and `docs/assurance/TIER-LAYER-MAP.md` already says no LLM verdict is an artefact: the tier gate never reads `/intent-check` output. The skills and agents that Crosscheck ships to other repositories still say the opposite:

- `/intent-check` Step 6 says the attestation "must be written before the commit that touches the protected files". Step 7 drafts a pre-commit hook that rejects any commit touching a protected file unless `.assurance/intent-check-attestation.json` exists, matches the files' hash, and records an LLM `pass`. Steps 0 and 5 describe the check as gating commits.
- `references/attestation-schema.md` opens by calling the attestation the record a pre-commit hook uses to allow a commit, and spends two sections on that hook.
- `/assurance-init` step 6.7d tells a target repository that Tier 3 needs "plan, intent-check attestation, and a governance-note block".
- `/protected-surface-amend` asks every Class A amendment to state whether "attestation regeneration" is required, and its review checklist asks that an "attestation / intent-check baseline refresh is queued".
- `hellebuyck` accepts a "prior attestation" as the authority for an amendment, and its guidelines say pre-commit hooks are "attestation checks, verifying that slow LLM pipelines actually ran".
- `add-orchestrator` hands off `/intent-check` "per PR touching protected surfaces", `lowry` names the intent check as the next step after green, and `draft-invariants` describes the attestation as "a fast pre-commit check".
- `docs/gates/intent-check-verdict.md` says the attestation "can gate a commit through a companion pre-commit hook", and `docs/gates/intent-check-kill-criterion.md` speaks of the check "gating commits".

A repository that follows these instructions makes an LLM verdict a commit gate.

## Proposed outcome
- `/intent-check` still runs the round trip, appends the tracker row and writes `.assurance/intent-check-attestation.json`. The skill and its schema call that file an advisory record of an LLM run, say that no gate, hook or reviewer may require it, and no longer draft a pre-commit hook that reads it.
- No skill or agent names the attestation, or an `/intent-check` run, as a Tier 3 artefact, an amendment authority, a review checklist item, or a required step.
- The two gate explainers say the verdict is advisory and gates nothing.
- The hash discipline that `add-orchestrator` and `draft-invariants` borrow for the session marker stays as it is, and they cite it by section name rather than by line numbers, which this change would break.

## Affected users and systems
- Users of the Crosscheck plugin in other repositories, who run `/intent-check`, `/assurance-init`, `/protected-surface-amend` and the `hellebuyck`, `add-orchestrator` and `lowry` agents.
- Files: `crosscheck/skills/intent-check/SKILL.md` and `references/attestation-schema.md`, `crosscheck/skills/assurance-init/SKILL.md`, `crosscheck/skills/protected-surface-amend/SKILL.md`, `crosscheck/skills/draft-invariants/SKILL.md`, `crosscheck/agents/hellebuyck.md`, `crosscheck/agents/add-orchestrator.md`, `crosscheck/agents/lowry.md`, `docs/gates/intent-check-verdict.md`, `docs/gates/intent-check-kill-criterion.md`, `docs/gates/README.md`.

## Constraints
- Roadmap item VA-1 governs the change. Every `SKILL.md` and `agents/*.md` edit is a Class A protected surface, so the change is Tier 3 and carries a governance note.
- `/intent-check` keeps its pipeline, its tracker schema, its kill criterion and its attestation schema. Only the claims about what the record is for change. Repositories that already read the tracker or the file keep working.
- The session-marker hash in `add-orchestrator` and `draft-invariants` must not change.
- `crosscheck/README.md` and `crosscheck/docs/assurance-hierarchy.md` belong to task VA-1.2. The example workflows under `crosscheck/docs/examples/workflows/`, which describe a "Mandatory L5 gate" that runs `/intent-check`, are not a skill or agent, so this change adds task VA-1.3 for them instead of changing them.

## Open questions
None. `docs/VISION.md` rule 1 and the "No LLM verdict is an artefact" paragraph of `docs/assurance/TIER-LAYER-MAP.md` settle the direction.
