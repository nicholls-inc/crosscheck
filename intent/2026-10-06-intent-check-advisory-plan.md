# Plan: Make the intent-check attestation an advisory record

Intent: `intent/2026-10-06-intent-check-advisory.md`
Spec: `intent/2026-10-06-intent-check-advisory-spec.md`
Governing roadmap item: VA-1. Task: VA-1.1. Tier: 3.

This plan is not the root `plan.md`, which belongs to an earlier change. The pull request body cites this file with a `Plan:` line.

## Order of work

1. Write the governance note `.assurance/protected-surface-amend/intent-check-advisory-2026-10-06.md`, naming every protected file below, and commit it with the intent, the spec and this plan. The protected-surface hook allows the edits only once the note is on the branch.
2. `crosscheck/skills/intent-check/SKILL.md` (IA-1 to IA-3). Reword the description, the opening paragraphs, the Step 0 refusal, the Step 5 `phase_verdict` bullet and Step 6 so the attestation is an advisory record. Delete Step 7, renumber Report and "What this does NOT catch" to Steps 7 and 8, and drop the hook line from the verification checklist.
3. `crosscheck/skills/intent-check/references/attestation-schema.md` (IA-4). Replace the opening and "Why an attestation" with a statement of what the record is for and that no gate reads it. Delete "Pre-commit hook" and "Registering the hook". Reword the hook mentions in the field table, the hash section, the `pipeline_output` section and the `/protected-surface-amend` section.
4. `crosscheck/skills/assurance-init/SKILL.md` step 6.7d (IA-5), and the roadmap principle it seeds about pre-commit hooks. `crosscheck/skills/assurance-status/SKILL.md` Step 2.3 stops offering the attestation as a way to find protected-surface edits.
5. `crosscheck/skills/protected-surface-amend/SKILL.md` Steps 3 and 7 (IA-6).
6. `crosscheck/agents/hellebuyck.md` (IA-7): the skill table row, the routing row for protected-surface PRs, the FP-tracker quality gate, the authority gate, and the "Attestation over trust" guideline.
7. `crosscheck/agents/add-orchestrator.md`, `crosscheck/agents/lowry.md`, `crosscheck/skills/draft-invariants/SKILL.md` (IA-8). Replace "lines 76–92" with the section name.
8. `docs/gates/intent-check-verdict.md`, `docs/gates/intent-check-kill-criterion.md`, `docs/gates/README.md` (IA-3, IA-9), and the line citation in `crosscheck/docs/orchestrator-coordination.md`.
9. `docs/TASKS.md`: set VA-1.1 to `done` with this intent as its record, and add VA-1.3 for the example workflows. Add a root `JOURNAL.md` entry.

## Files

| File | Protected | Change |
|---|---|---|
| `crosscheck/skills/intent-check/SKILL.md` | yes | attestation advisory, Step 7 removed |
| `crosscheck/skills/assurance-init/SKILL.md` | yes | Tier 3 artefact list |
| `crosscheck/skills/protected-surface-amend/SKILL.md` | yes | attestation follow-ups removed |
| `crosscheck/skills/draft-invariants/SKILL.md` | yes | section reference, record description |
| `crosscheck/agents/hellebuyck.md` | yes | five lines |
| `crosscheck/agents/add-orchestrator.md` | yes | hand-off wording, section references |
| `crosscheck/agents/lowry.md` | yes | intent check is optional, not the judgement |
| `crosscheck/skills/assurance-status/SKILL.md` | yes | Step 2.3 stops reading the attestation |
| `crosscheck/skills/intent-check/references/attestation-schema.md` | no | hook sections removed |
| `docs/gates/intent-check-verdict.md`, `docs/gates/intent-check-kill-criterion.md`, `docs/gates/README.md`, `crosscheck/docs/orchestrator-coordination.md` | no | wording, line pointers |
| `docs/TASKS.md`, `JOURNAL.md` | no | record |
| `.assurance/protected-surface-amend/intent-check-advisory-2026-10-06.md` | no | governance note |
| `intent/2026-10-06-intent-check-advisory*.md` | no | stage artefacts |

## Risks

- **A user who installed the old draft hook keeps it.** The skill never installed the hook, so only a repository that applied the draft by hand has it. The change cannot reach those copies. The attestation schema is unchanged, so such a hook keeps working until its owner removes it.
- **Broken references.** Other files cite line numbers in `attestation-schema.md` and step numbers in the intent-check `SKILL.md`. Step 7 of the plan replaces the line citations with a section name, and step 8 updates the gate index. The `grep` below finds any that remain.
- **Conformance oracle.** `crosscheck/conformance` checks skill structure and routing tokens. Removing a step adds or removes no routing token, but the oracle runs as part of the proof.

## Proof that it worked

- `git grep -n -i -E "must be written \*\*before\*\* the commit|stops gating commits|keeps gating commits|attestation hook refuses|prior attestation|Attestation over trust|attestation regeneration|baseline refresh|intent-check attestation, and|can gate a commit|lines 76" -- crosscheck/skills crosscheck/agents crosscheck/docs/orchestrator-coordination.md docs/gates` prints nothing. Run on `origin/main`, the same command prints the lines this change targets.
- `git grep -n -i "pre-commit" -- crosscheck/skills/intent-check` prints only lines that say no hook reads the record.
- `go run ./crosscheck/conformance crosscheck` exits 0 before and after the change.
- `node scripts/ci/task-queue.mjs check` passes with `Task: VA-1.1`, and the Tier Gate passes at Tier 3 on the pull request.
