# Spec: The intent-check attestation is an advisory record

Intent: `intent/2026-10-06-intent-check-advisory.md`
Governing roadmap item: VA-1. Task: VA-1.1.

The behaviour under change is what Crosscheck's skills and agents tell an agent to do. Each statement below is checkable by reading the named file after the change, and the `grep` in the plan checks the ones a pattern can catch.

## Statements

- **IA-1.** `crosscheck/skills/intent-check/SKILL.md` does not instruct the agent to draft, install or recommend a pre-commit hook, CI job or other gate that reads `.assurance/intent-check-attestation.json`. The former Step 7 is removed, and the steps after it are renumbered.
- **IA-2.** `crosscheck/skills/intent-check/SKILL.md` does not say that the attestation must exist, or must record `pass`, before a commit, a merge or a protected-surface edit. It states that the attestation is an advisory record of an LLM run, that it is not evidence, and that no gate, hook or reviewer may require it, citing `docs/VISION.md` rule 1.
- **IA-3.** The kill-criterion refusal in Step 0 and the gate explainer `docs/gates/intent-check-kill-criterion.md` describe retirement as "stop running the check" and not as "stop gating commits". The kill criterion, its thresholds and its tracker schema are unchanged.
- **IA-4.** `crosscheck/skills/intent-check/references/attestation-schema.md` keeps the JSON schema and the SHA-256 computation. It drops the pre-commit hook pseudocode and the hook registration section, and it states that no gate reads the file.
- **IA-5.** `crosscheck/skills/assurance-init/SKILL.md` step 6.7d lists the Tier 3 artefacts as a plan and a governance-note block for protected-surface edits, and states that no LLM verdict, such as an `intent-check` attestation, is a tier artefact.
- **IA-6.** `crosscheck/skills/protected-surface-amend/SKILL.md` does not ask an amendment to state, queue or check an attestation regeneration or an `intent-check` baseline refresh.
- **IA-7.** `crosscheck/agents/hellebuyck.md` does not accept an attestation as an amendment authority, and does not describe pre-commit hooks as checks that an LLM pipeline ran. It describes the `/intent-check` record as advisory.
- **IA-8.** `crosscheck/agents/add-orchestrator.md`, `crosscheck/agents/lowry.md` and `crosscheck/skills/draft-invariants/SKILL.md` present `/intent-check` as an optional search that points at likely gaps, never as a required step or as the judgement of intent. They cite the hash discipline by its section heading, "SHA-256 computation (exact)", and the session-marker hash algorithm does not change.
- **IA-9.** `docs/gates/intent-check-verdict.md` does not say the attestation can gate a commit. `docs/gates/README.md` points rows 3 and 4 at the current step numbers of `crosscheck/skills/intent-check/SKILL.md`.
- **IA-10.** `crosscheck/docs/agents.md`, step 4 of the adoption order in `crosscheck/docs/assurance-hierarchy.md`, `docs/gates/lowry-drift-packet.md`, `crosscheck/docs/add/phase4-design-decisions.md` and the prose of `crosscheck/skills/intent-check/references/fp-tracker-schema.md` do not present `/intent-check` as a routine step, a gate or the judgement of intent. Step 6 of the skill also points at the draft hook files an earlier version left behind.

## Not changed

- The round-trip prompts, the semantic validation rules, the FP tracker, the attestation schema fields and the hash algorithm.
- `crosscheck/README.md` and the "not yet reached" wording in `crosscheck/docs/assurance-hierarchy.md` (task VA-1.2), and the example workflows under `crosscheck/docs/examples/workflows/` (task VA-1.3, added by this change).

## Unresolved concerns

- Nothing deterministic keeps a future edit from reintroducing a required attestation. A conformance rule that scans skills and agents for such phrasing is not yet reached. The property that blocks it is a phrase list that catches the claim without catching the advisory wording this change adds, and the open question is whether a wording lint can tell the two apart.
