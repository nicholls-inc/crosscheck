# Spec: Say "not yet reached" in Crosscheck's skills and agents

Intent: `intent/2026-10-06-not-yet-reached-skills.md`. Task: VA-1.5.

## Requirements

- **NR-1.** No line in `crosscheck/agents/**` or `crosscheck/skills/**` describes a layer or a class of code as "best-effort", "out of scope" or "not addressed". The check is `grep -rniE 'best[- ]effort|out[- ]of[- ]scope|not addressed|not addressable' crosscheck/agents crosscheck/skills`, and every remaining hit is on the keep list below.
- **NR-2.** Layer 6 is described the same way everywhere it was called "best-effort": a proof that a spec is complete is not yet reached; the blocking property is that no formal requirement is tied to the spec; the open question is how to write requirements formally and prove that a spec achieves them (RQ-1). Skill output on Layer 6 is called search, and its findings point at gaps but are not evidence that none remain.
- **NR-3.** `assurance-layer-audit` says "Not yet reached" where it said "Not addressable" for Layer 2: the Go, interpreted-language and VM bullets, the reach table row and the verification checklist. Each names the blocking property (the compiler, interpreter or VM is in the trusted base) and the open question (whether translation validation of each compiled or extracted file can replace that trust, as VA-1.2 put it). The reach table gives Layer 6 "Search only; proof not yet reached" and cites RQ-1. "Not addressable" is the same position as "not addressed", so NR-1's grep includes it.
- **NR-4.** `audit-invariant-consistency` blind spot 2 says spec-internal contradictions are not yet reached by the skill. Blocking property: the skill treats each spec section as authoritative and compares invariants to it, so it never compares two sections. Open question: how to check a spec's own consistency, which a solver can decide once the spec is formal.
- **NR-5.** `drt-oracle` says the Aeneas route for Rust is not yet reached. Blocking property: Crosscheck integrates no Charon and Aeneas toolchain, so Rust source has no mechanically derived Lean model. Open question: whether a Lean model derived from the Rust source can serve as the DRT oracle, when the oracle is meant to be written independently of the code it checks.
- **NR-6.** `acceptance-oracle-draft` reports a flow it cannot check mechanically as not yet reached by the oracle, under a section titled `## Rejected Flows (not yet reached by this oracle)`. The example row "Dashboard looks nice" names the blocking property (no programmatic observable) and the vision's open question for user interfaces (how to specify "looks right").
- **NR-7.** `intent-check` Step 9 calls the pipeline a Layer 5 search tool whose verdict is an LLM's judgement and not evidence.
- **NR-9.** `rationale` marks a `[STATIC]` leaf `Read (static; LLM reading, not evidence)` and leaves its checkbox open until a deterministic check or a human confirms it. The words `Verified (static)` do not appear. Blocking property for a deterministic check: a static claim is prose, so no checker is tied to it. Open question: which static claims can be restated as a check that a tool decides (type checker, linter, AST query, CGV extraction).
- **NR-10.** `rationale`'s summary table has the columns evidence in hand (proved or tested), read only, and pending. A `[FORMAL]` leaf counts as evidence only after byfuglien's pipeline proves it, and a `[BEHAVIORAL]` leaf only after its tests run. The word `Verified` does not head a column.
- **NR-11.** `rationale` does not say the root claim "holds by construction". It says the claim is supported by the evidence and the tree does not prove it. A proof that the tree covers every requirement is not yet reached. Blocking property: an LLM drew the decomposition and no formal requirement ties the leaves to the requirements. Open question: RQ-1.
- **NR-12.** `acceptance-oracle-draft`'s description does not say it "measures whether the spec was the right spec". It checks user-observable behaviour against scenarios a human approves. A proof that the spec was the right spec is not yet reached (same blocking property and open question as NR-2).
- **NR-8.** No skill gains or loses a step, a section, a routing token, a threshold or a kill criterion. `go run ./crosscheck/conformance crosscheck` gives the same result before and after. NR-9 to NR-12 change the strings a `/rationale` report and a `/acceptance-oracle-draft` description show, and add no step or section.

## Keep list

These hits mean something else and stay:

- `assurance-status/SKILL.md`: a best-effort grep of test files.
- `informal-spec/SKILL.md` (three hits): the module boundary lists functions outside the module's contract.
- `spec-iterate/SKILL.md`: a requirement the spec chooses not to formalise.
- `intent-check/SKILL.md` Step 2 and `intent-check/references/round-trip-prompt.md`: the carve-out vocabulary that the diff-checker scans invariant prose for.
- `journal-context/docs/invariants/journal-context.md`: the v1 walk stays inside one repository. That is a limit of a context loader, not a class of code a verifier does not reach.

## Concerns flagged

- The blocking properties and open questions in NR-4, NR-5, NR-9 and NR-11 are not in the vision's class table. This spec proposes them.
- No check enforces NR-1 in CI. A wording lint for skills and agents is not yet reached. The property that blocks it is that the phrases are fine in some senses (the keep list), so a plain grep cannot tell a violation from a keep. The open question is whether a keep-list file plus a grep is precise enough to run as a gate.
- Layer 5's "probabilistic" and "~96%" in `hellebuyck.md` and `assurance-layer-audit`, and Layer 3's "aspirational" in `assurance-layer-audit`, still contradict rule 1 and the scope section. Row VA-1.8 owns them.
