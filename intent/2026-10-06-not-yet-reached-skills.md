# Intent: Say "not yet reached" in Crosscheck's skills and agents

Task: VA-1.5. Governing roadmap item: VA-1.

## Problem statement
`docs/VISION.md` says no class of code is outside the vision. When a tool does not reach a class, it must say "not yet reached", name the property that blocks it, and name the open question. The vision also says spec completeness is provable relative to a formal requirement, and rule 1 makes every LLM-based check a search tool.

VA-1.2 brought `crosscheck/README.md` and `crosscheck/docs/assurance-hierarchy.md` in line. The skills and agents still state the old positions, and they are what an agent reads and repeats to a user:

1. `crosscheck/agents/hellebuyck.md` calls Layer 6 and `/spec-adversary` "best-effort" four times, including a quality gate that tells every audit skill to label its output "best-effort".
2. `audit-spec-coverage`, `audit-invariant-consistency`, `spec-adversary` and `assurance-layer-audit` call themselves or Layer 6 "best-effort". `assurance-layer-audit` also writes a reach table into its report that gives Layer 6 the reach "Best-effort" and Layer 2 "Not addressable".
3. `audit-invariant-consistency` calls spec-internal contradictions "out of scope".
4. `drt-oracle` calls the Aeneas route for Rust code "explicitly out of scope".
5. `acceptance-oracle-draft` reports flows with no programmatic observable, such as "Dashboard looks nice", as "out of scope for this oracle". User interfaces are a class in the vision's class table.
6. `intent-check` calls its pipeline "Layer 5 best-effort".

A user who reads these reports concludes that Crosscheck has given up on these classes and layers, which the vision says is false.

## Proposed outcome
Each of these statements says "not yet reached", names the blocking property and names the open question. Layer 6 uses the wording VA-1.2 put in the README: a proof of completeness is not yet reached because no formal requirement is tied to the spec, and the open question is how to write requirements formally and prove that a spec achieves them (roadmap item RQ-1). The Layer 6 skills call their output search results that point at gaps, not evidence. User interfaces take the vision's open question, "how to specify 'looks right'". Spec-internal contradictions and the Aeneas route take a blocking property and an open question this change proposes.

Matches that mean something else stay as they are: a best-effort grep, a module boundary that lists functions outside a module's contract, a requirement the spec does not formalise, the carve-out vocabulary that `/intent-check`'s prompt scans for, and the v1 limit of `/journal-context` to one repository, which is a limit of a context loader, not a class of code.

## Affected users and systems
- Anyone who runs the Layer 6 skills, `/assurance-layer-audit`, `/drt-oracle`, `/acceptance-oracle-draft`, `/intent-check` or the `hellebuyck` agent, and reads what they report.
- Eight Class A protected files under `crosscheck/agents/` and `crosscheck/skills/`, plus `docs/TASKS.md`, `crosscheck/JOURNAL.md` and a governance note. The change is Tier 3.

## Constraints
- Only wording changes. No skill gains or loses a step, an output section, a routing token or a threshold. The `## Rejected Flows` heading keeps its first two words, which `acceptance-oracle-draft/references/scenario-schema.md` cites.
- VA-1.1 (open PR #72) edits the attestation wording in `hellebuyck.md` and `intent-check/SKILL.md`. This change touches other lines of both files.
- Layer 5's strength ("probabilistic", "~96%") in the skills and agents is not one of the three phrases. This change adds row VA-1.8 for it and for the Layer 3 reach "aspirational" in `assurance-layer-audit`, and does not edit them. VA-1.3 and VA-1.7 are taken by open PRs #72 and #76.
- Docs under `crosscheck/docs/` belong to VA-1.6.

## Open questions
None. The proposed blocking properties and open questions for spec-internal contradictions and the Aeneas route are recorded in the spec, and the maintainer ratifies them by merging.
