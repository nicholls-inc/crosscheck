# Spec: Say "not yet reached" in Crosscheck's skills and agents

Intent: `intent/2026-10-06-not-yet-reached-skills.md`. Task: VA-1.5.

## Requirements

- **NR-1.** No line in `crosscheck/agents/**` or `crosscheck/skills/**` describes a layer or a class of code as "best-effort", "out of scope" or "not addressed". The check is `grep -rniE 'best[- ]effort|out[- ]of[- ]scope|not addressed' crosscheck/agents crosscheck/skills`, and every remaining hit is on the keep list below.
- **NR-2.** Layer 6 is described the same way everywhere it was called "best-effort": a proof that a spec is complete is not yet reached; the blocking property is that no formal requirement is tied to the spec; the open question is how to write requirements formally and prove that a spec achieves them (RQ-1). Skill output on Layer 6 is called search, and its findings point at gaps but are not evidence that none remain.
- **NR-3.** `assurance-layer-audit`'s reach table gives Layer 2 "Not yet reached (compiler and runtime in the trusted base)" and Layer 6 "Search only; proof not yet reached (RQ-1)".
- **NR-4.** `audit-invariant-consistency` blind spot 2 says spec-internal contradictions are not yet reached by the skill. Blocking property: the skill treats each spec section as authoritative and compares invariants to it, so it never compares two sections. Open question: how to check a spec's own consistency, which a solver can decide once the spec is formal.
- **NR-5.** `drt-oracle` says the Aeneas route for Rust is not yet reached. Blocking property: Crosscheck integrates no Charon and Aeneas toolchain, so Rust source has no mechanically derived Lean model. Open question: whether a Lean model derived from the Rust source can serve as the DRT oracle, when the oracle is meant to be written independently of the code it checks.
- **NR-6.** `acceptance-oracle-draft` reports a flow it cannot check mechanically as not yet reached by the oracle, under a section titled `## Rejected Flows (not yet reached by this oracle)`. The example row "Dashboard looks nice" names the blocking property (no programmatic observable) and the vision's open question for user interfaces (how to specify "looks right").
- **NR-7.** `intent-check` Step 9 calls the pipeline a Layer 5 search tool whose verdict is an LLM's judgement and not evidence.
- **NR-8.** No skill gains or loses a step, a section, a routing token, a threshold or a kill criterion. `go run ./crosscheck/conformance crosscheck` gives the same result before and after.

## Keep list

These hits mean something else and stay:

- `assurance-status/SKILL.md`: a best-effort grep of test files.
- `informal-spec/SKILL.md` (three hits): the module boundary lists functions outside the module's contract.
- `spec-iterate/SKILL.md`: a requirement the spec chooses not to formalise.
- `intent-check/SKILL.md` Step 2 and `intent-check/references/round-trip-prompt.md`: the carve-out vocabulary that the diff-checker scans invariant prose for.
- `journal-context/docs/invariants/journal-context.md`: the v1 walk stays inside one repository. That is a limit of a context loader, not a class of code a verifier does not reach.

## Concerns flagged

- The blocking properties and open questions in NR-4 and NR-5 are not in the vision's class table. This spec proposes them.
- No check enforces NR-1 in CI. A wording lint for skills and agents is not yet reached. The property that blocks it is that the phrases are fine in some senses (the keep list), so a plain grep cannot tell a violation from a keep. The open question is whether a keep-list file plus a grep is precise enough to run as a gate.
- Layer 5's "probabilistic" and "~96%" in `hellebuyck.md` and `assurance-layer-audit`, and Layer 3's "aspirational" in `assurance-layer-audit`, still contradict rule 1 and the scope section. Row VA-1.8 owns them.
