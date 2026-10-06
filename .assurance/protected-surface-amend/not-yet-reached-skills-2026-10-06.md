## Protected-Surface Amendment

**Target file(s):** `crosscheck/agents/hellebuyck.md` (+ 7 others, see Diff Plan)
**Class:** A (skill and agent behaviour definitions)
**Matched rule:** `crosscheck/skills/*/SKILL.md`, `crosscheck/agents/*.md`
**Date:** 2026-10-06

### Change Description

1. `crosscheck/agents/hellebuyck.md`: the skill table, the Task Classification intro, the "Best-effort honesty" quality gate and the Layer 6 guideline describe Layer 6 as search, with a proof of completeness not yet reached because no formal requirement is tied to the spec (RQ-1).
2. `crosscheck/skills/audit-spec-coverage/SKILL.md`: the Layer 6 note and the Step 9 intro say the same.
3. `crosscheck/skills/audit-invariant-consistency/SKILL.md`: the layer note, the comparison row, the closing note of "What this does NOT catch" and the Step 9 intro say the same. Blind spot 2 says spec-internal contradictions are not yet reached, with a blocking property and an open question.
4. `crosscheck/skills/spec-adversary/SKILL.md`: the frontmatter description and the Step 7 intro say the same.
5. `crosscheck/skills/assurance-layer-audit/SKILL.md`: the Layer 6 reach paragraph, the "Layer 6 work last" heuristic, and the Layer 2 and Layer 6 rows of the reach table the skill writes into its report.
6. `crosscheck/skills/drt-oracle/SKILL.md`: the Aeneas reference says the Rust route is not yet reached, with a blocking property and an open question.
7. `crosscheck/skills/acceptance-oracle-draft/SKILL.md`: a flow with no programmatic observable is reported as not yet reached by the oracle, under `## Rejected Flows (not yet reached by this oracle)`. The "Dashboard looks nice" example names the vision's open question for user interfaces.
8. `crosscheck/skills/intent-check/SKILL.md`: the Step 9 intro calls the pipeline a Layer 5 search tool whose verdict is not evidence.

### Rationale

Task VA-1.5. `docs/VISION.md` says no class of code is outside the vision: a tool that does not reach a class says "not yet reached", names the blocking property and names the open question. It also says spec completeness is provable relative to a formal requirement, and rule 1 makes LLM-based checks search tools. VA-1.2 aligned the README and the hierarchy guide. The skills and agents still told users that Layer 6 is "best-effort" and that some classes are "out of scope". Intent: `intent/2026-10-06-not-yet-reached-skills.md`. Spec: `intent/2026-10-06-not-yet-reached-skills-spec.md`. Plan: `intent/2026-10-06-not-yet-reached-skills-plan.md`.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (immediate horizon, item VA-1)
- **Title:** Bring what Crosscheck tells its users in line with the vision
- **Scope coverage:** VA-1's scope says the skills and agents "call spec completeness 'best-effort', and they call some classes of code 'out of scope' or 'not addressed'". Its acceptance asks that each class a tool does not reach be described as "not yet reached", with the blocking property and the open question. Task VA-1.5 in `docs/TASKS.md` is this change.

### Authority

- **Authoriser:** harry-nicholls. The merge is the approval.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `crosscheck/agents/hellebuyck.md` | Verification skill table, Task Classification, quality gates, Specification chain guidelines | reworded |
| 2 | `crosscheck/skills/audit-spec-coverage/SKILL.md` | Description, Step 9 | reworded |
| 3 | `crosscheck/skills/audit-invariant-consistency/SKILL.md` | layer note, "Distinct from /intent-check" table, Step 8 blind spot 2 and closing note, Step 9 | reworded |
| 4 | `crosscheck/skills/spec-adversary/SKILL.md` | frontmatter description, Step 7 | reworded |
| 5 | `crosscheck/skills/assurance-layer-audit/SKILL.md` | Step 4 Layer 6, Step 5 reach table (Layer 2, Layer 6), Step 6 heuristics | reworded |
| 6 | `crosscheck/skills/drt-oracle/SKILL.md` | References, Aeneas bullet | reworded |
| 7 | `crosscheck/skills/acceptance-oracle-draft/SKILL.md` | CRUCIAL RULE, Step 7 | reworded; section heading text changed |
| 8 | `crosscheck/skills/intent-check/SKILL.md` | Step 9 intro | reworded |

### Test / Coverage Impact

- No invariant, test or eval changes. No skill gains or loses a step, an output section, a routing token, a threshold or a kill criterion, so `go run ./crosscheck/conformance crosscheck` is unaffected.
- Output changes a user can see: `/assurance-layer-audit`'s reach table reads "Not yet reached" for Layer 2 and "Search only" for Layer 6, and `/acceptance-oracle-draft`'s rejected-flows heading reads `## Rejected Flows (not yet reached by this oracle)`. A consumer that matched the old heading text in full breaks; `references/scenario-schema.md` cites only `## Rejected Flows`, which is unchanged.
- No check enforces the new wording. A wording lint for skills and agents is not yet reached; the spec records the blocking property and the open question.

### Review Checklist

- [x] Rationale is anchored to the scope section and rule 1 of `docs/VISION.md`.
- [ ] REQUIRES HUMAN VERIFICATION: Authoriser is a named human.
- [ ] REQUIRES HUMAN VERIFICATION: VA-1 covers the skills and agents that call Layer 6 or a class of code "best-effort", "out of scope" or "not addressed".
- [x] The diff plan names every changed protected file.
- [ ] REQUIRES HUMAN VERIFICATION: The blocking properties and open questions proposed for spec-internal contradictions (`audit-invariant-consistency`) and the Aeneas route (`drt-oracle`) are right. Neither is in the vision's class table.
- [ ] REQUIRES HUMAN VERIFICATION: The keep list in the spec is right: each remaining "best-effort" or "out of scope" hit in `crosscheck/agents` and `crosscheck/skills` means something other than a layer or a class of code.
- [ ] REQUIRES HUMAN VERIFICATION: No check is weakened. Only prose changes; no deterministic check reads these files.
