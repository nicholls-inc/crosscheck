# Intent: Say "not yet reached" in the rest of Crosscheck's docs

Task: VA-1.6. Governing roadmap item: VA-1.

## Problem statement
VA-1.2 changed `crosscheck/README.md` and `crosscheck/docs/assurance-hierarchy.md` to say "not yet reached", with the blocking property and the open question, where they had called a class of code or a layer "out of scope", "not addressed" or "best-effort". Other docs under `crosscheck/docs/` still state the old positions:

1. `crosscheck/docs/research/assurance-hierarchy.md`, which both files link to for "the full treatment". Its Layer 6 section says "This layer is best-effort" and "There is no theorem that can prove a specification is complete". `docs/VISION.md` says completeness is provable relative to a formal requirement. Its specification-chain summary ends at "best-effort (Layer 6)". Its "What this hierarchy is not good for" section calls performance, partition failures and security "out of scope for the hierarchy entirely". Its Layer 1 reach paragraph says concurrent and effectful code "falls outside the verifiable surface". Its Layer 2 reach paragraph calls Layer 2 "Aspirational". Its Scope section says the hierarchy "explicitly excludes" whether the spec solves the right problem.
2. The reference workflows `crosscheck/docs/examples/workflows/example.md`, `tier-b/assurance-squad.md` and `tier-b/assurance-pr-gate.md` label Layer 6, or Layer 5 and 6 output, "best-effort". `assurance-squad.md` tells its agent to put the label `Layer 6 (best-effort)` on every issue it opens.
3. `crosscheck/docs/orchestrator-coordination.md` requires an honesty section in "every probabilistic or best-effort findings file".

## Proposed outcome
Each statement in the list above says "not yet reached", names the blocking property and names the open question, or, for skill output, calls it a search whose findings point at gaps and are not evidence. The wording matches VA-1.2's README section "Where Crosscheck does not reach yet" and its Layer 2, 3 and 6 rows, so the guide and its "full treatment" agree. Layer 6 points at roadmap item RQ-1. The open questions come from the class table in `docs/VISION.md` where one fits.

These files are left as written, because each is a dated record or a retrospective of what was believed when it was written:

- `crosscheck/docs/research/crosscheck-review-may-2026.md`, `perplexity-crosscheck-analysis-may-2026.md`, `phase-roadmap-may-2026.md` and `literature-review.md`;
- `crosscheck/docs/research/adr/0001-behavioral-specs-at-layer-4.md`, a decision record;
- everything under `crosscheck/docs/add/.retrospective/` and `crosscheck/docs/reports/`.

These matches mean something other than a class of code or a layer, so they stay: "out of scope for this analysis" in `logic-distribution-analysis.md`, "Excluded: fully Dafny-verified slices" in `crosscheck-tla-vgd-addendum.md` (DRT is redundant there, not unreached), "out-of-scope latent risk" in `add/orchestrator-improvements.md`, a best-effort cache key and a best-effort count in the tier-b workflows, "excluded from both numerator and denominator" in the workflows README, the `excluded` paths in `invariants/shouldExclude.md`, and "out of scope for I1–I7" in `invariants/parseDafnyOutput.md`, which names an input format that no invariant covers.

## Affected users and systems
- Anyone who follows the hierarchy guide's link to the full treatment, and anyone who copies the reference workflows into their own repository.
- `crosscheck/docs/research/assurance-hierarchy.md`, `crosscheck/docs/examples/workflows/example.md`, `crosscheck/docs/examples/workflows/tier-b/assurance-squad.md`, `crosscheck/docs/examples/workflows/tier-b/assurance-pr-gate.md`, `crosscheck/docs/orchestrator-coordination.md`, `crosscheck/JOURNAL.md` and `docs/TASKS.md`. None is protected. No skill, agent or code changes, so the change is Tier 1.

## Constraints
- No file under `crosscheck/docs/invariants/` changes, so no governance note is needed.
- The research doc gives Layer 5 the confidence "probabilistic" with "~96% accuracy". Row VA-1.7, added by open PR #76, owns that wording. This change edits only the Layer 6 half of the specification-chain summary.
- Row VA-1.3, added by open PR #72, owns the example workflows' treatment of `/intent-check` as a mandatory gate. This change edits only their "best-effort" labels.

## Open questions
None. The blocking properties and open questions are the ones VA-1.2 used, and the maintainer ratified them by merging #71.
