## Protected-Surface Amendment

**Target file(s):** `crosscheck/skills/intent-check/SKILL.md` (+ 7 others, see Diff Plan)
**Class:** A (skill and agent behaviour definitions)
**Matched rule:** `crosscheck/skills/*/SKILL.md`, `crosscheck/agents/*.md`
**Date:** 2026-10-06

### Change Description

1. `crosscheck/skills/intent-check/SKILL.md`: the description, the opening paragraphs, the Step 0 refusal, the Step 5 `phase_verdict` bullet and Step 6 describe `.assurance/intent-check-attestation.json` as an advisory record of an LLM run that no gate, hook or reviewer may require. Step 7, which drafted a pre-commit hook that rejects a commit without a passing attestation, is removed. Report and "What this does NOT catch" become Steps 7 and 8. The verification checklist drops the hook line. Step 6 gains a one-line `grep` that finds a hook an earlier version of the skill drafted, and tells the user to remove it. The Step 7 banner and the checklist still offer classifying a verdict `spurious` in the tracker, which the tracker schema already defined; the kill-criterion arithmetic is unchanged.
2. `crosscheck/skills/assurance-init/SKILL.md`: step 6.7d lists the Tier 3 artefacts as a plan and a governance-note block, and says no LLM verdict is a tier artefact. The roadmap principle it seeds says pre-commit hooks run deterministic checks and never require an LLM verdict, in place of "fast attestation checks".
3. `crosscheck/skills/protected-surface-amend/SKILL.md`: Steps 3 and 7 no longer ask a Class A amendment to state or queue an attestation regeneration or an `intent-check` baseline refresh.
4. `crosscheck/skills/draft-invariants/SKILL.md`: cites the hash discipline by section name, and describes the attestation as an advisory record rather than a pre-commit check.
5. `crosscheck/agents/hellebuyck.md`: the skill table, the routing row, the FP-tracker gate, the authority gate (no "prior attestation") and the "Attestation over trust" guideline are reworded.
6. `crosscheck/agents/add-orchestrator.md`: the hellebuyck hand-off offers `/intent-check` as an optional search, and the hash discipline is cited by section name.
7. `crosscheck/agents/lowry.md`: the completion disclaimer and the green hand-off name intent as a human judgement that `/intent-check` and `/rationale` can inform, not decide.
8. `crosscheck/skills/assurance-status/SKILL.md`: Step 2.3 finds protected-surface edits from `git log` only, no longer from the attestation.

### Rationale

Task VA-1.1. `docs/VISION.md` rule 1 says no guarantee rests on the judgement of an LLM, and `docs/assurance/TIER-LAYER-MAP.md` says no LLM verdict is an artefact. The skills and agents still told a target repository to require a passing `intent-check` attestation before a commit, to list it as a Tier 3 artefact, to accept it as an amendment authority and to queue its refresh on every Class A amendment. Intent: `intent/2026-10-06-intent-check-advisory.md`. Spec: `intent/2026-10-06-intent-check-advisory-spec.md`. Plan: `intent/2026-10-06-intent-check-advisory-plan.md`.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (immediate horizon, item VA-1)
- **Title:** Bring what Crosscheck tells its users in line with the vision
- **Scope coverage:** VA-1's scope names the skills and agents that "tell a target repository that Tier 3 needs an `intent-check` attestation", and its acceptance asks that no skill or agent present an LLM verdict as a required artefact. Task VA-1.1 in `docs/TASKS.md` is this change.

### Authority

- **Authoriser:** harry-nicholls. The merge is the approval.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `crosscheck/skills/intent-check/SKILL.md` | description, Description, Steps 0, 5, 6, 7, 8, checklist | reworded; Step 7 (hook drafting) removed; Report and "What this does NOT catch" renumbered to Steps 7 and 8 |
| 2 | `crosscheck/skills/assurance-init/SKILL.md` | roadmap principles, 6.7d | reworded |
| 3 | `crosscheck/skills/protected-surface-amend/SKILL.md` | Step 3, Step 7 template | attestation follow-ups removed |
| 4 | `crosscheck/skills/draft-invariants/SKILL.md` | §1c marker validation and coordination note | reworded |
| 5 | `crosscheck/agents/hellebuyck.md` | skill table, routing table, quality gates, Governance guidelines | reworded |
| 6 | `crosscheck/agents/add-orchestrator.md` | Step 5 marker, hellebuyck hand-off, checklist | reworded |
| 7 | `crosscheck/agents/lowry.md` | completion contract, terminal states | reworded |
| 8 | `crosscheck/skills/assurance-status/SKILL.md` | Step 2.3 | attestation cross-reference removed |

### Test / Coverage Impact

- No invariant, test or eval changes. No skill gains or loses a routing token, so `go run ./crosscheck/conformance crosscheck` is unaffected.
- The `/intent-check` pipeline, tracker schema, kill criterion, attestation schema and hash algorithm are unchanged. The session-marker hash that `add-orchestrator` and `draft-invariants` compute is unchanged.
- No check enforces the new wording. A wording lint for skills and agents is not yet reached; the spec records the open question.

### Review Checklist

- [x] Rationale is anchored to `docs/VISION.md` rule 1 and the TIER-LAYER-MAP paragraph.
- [x] Authoriser is a named human.
- [x] VA-1 covers skills and agents that present an LLM verdict as a required artefact.
- [x] The diff plan names every changed protected file.
- [ ] REQUIRES HUMAN VERIFICATION: No check is weakened. A commit gate on an LLM verdict is removed from a draft the skill wrote; no deterministic check changes.
- [ ] REQUIRES HUMAN VERIFICATION: A repository that applied the old draft hook by hand keeps it until its owner acts. Step 6 of `/intent-check` and the attestation schema now tell the user to remove such a hook through `/protected-surface-amend`.
