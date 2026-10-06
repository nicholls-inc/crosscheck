# Intent: Say what Layers 4 to 6 prove, test or only search

Task: VA-1.4. Governing roadmap item: VA-1.

## Problem statement
Rule 1 of `docs/VISION.md` says `/intent-check` and other tools that use an LLM to find likely problems are search tools, and their output is never evidence. Rule 7 says every claim names its strength: proved, tested, observed or judged.

Two of the first files a new user reads still say otherwise:

1. `crosscheck/docs/assurance-hierarchy.md` says in its TL;DR that Layers 4 to 6 "prove the specification is the right specification", probabilistic at Layer 5 "(~96% accuracy on round-trip checks)". Its table gives Layer 5 the confidence "Probabilistic (~96%)" and Layer 4 the confidence "Deterministic (4); semi-formal (rationales)".
2. `crosscheck/README.md` calls `/intent-check` "round-trip intent verification" with "~96% accuracy", and says Hellebuyck's layers run "deterministic → probabilistic → search".

A reader concludes that a Layer 5 run proves a spec matches intent 96% of the time. It does not. The ~96% is an accuracy reported for the round-trip method on a development benchmark, and nothing in Layers 4 to 6 proves that a spec is the right one.

## Proposed outcome
Both files say, for each of Layers 4 to 6, what it proves, what it tests and what it only searches:

- **Layer 4.** `/check-regressions` re-proves the Dafny source of each changed hard-constraint spec, so it proves that much. It does not prove the extracted code still matches, and its soft constraints are property tests. `/invariant-coverage-scaffold` checks deterministically that each non-aspirational invariant is referenced by a comment in a test file. That check does not show a test checks its invariant. The covering tests and `/assurance-probe`'s mutation probes test, on the inputs they run. `/rationale` proves and tests nothing itself. It hands formal leaves to byfuglien's pipelines (Dafny proves, the Lean DRT oracle tests), writes behavioural leaves as tests for CI to run, leaves semantic leaves to human judgment, and only searches with its static leaves, which an LLM checks by reading code.
- **Layer 5.** Spec–intent alignment is search only. `/intent-check` and `/audit-invariant-consistency` are LLM pipelines. The ~96% is the accuracy that Claimcheck reports for the round-trip method on its development benchmark (`crosscheck/docs/research/assurance-hierarchy.md`). It measures how often the search is right, not how strongly a spec matches intent, and `/intent-check`'s accuracy on real pull requests is unmeasured. `/acceptance-oracle-draft` drafts scenarios that a human approves and CI runs, so an approved scenario tests. A proof that a spec achieves its intent is not yet reached. The blocking property is that no formal requirement is tied to the spec, and the open question is RQ-1's.
- **Layer 6.** Search only, as the files already say.

The grep `Probabilistic \(~96%\)|prove the specification is the right` finds nothing in either file.

## Affected users and systems
- Anyone deciding how far to trust a Layer 4, 5 or 6 result.
- `crosscheck/README.md`, `crosscheck/docs/assurance-hierarchy.md`, `crosscheck/JOURNAL.md` and `docs/TASKS.md`. None is protected, and no behaviour changes, so the change is Tier 1.

## Constraints
- No skill, agent or code changes. VA-1.1 (open PR #72) owns the `intent-check` wording in skills and agents, and rewrites step 4 of the onboarding flow in `crosscheck/docs/assurance-hierarchy.md`. This change leaves that step alone so the two pull requests do not conflict.
- `crosscheck/docs/research/assurance-hierarchy.md` holds the same Layer 5 claim. VA-1.6 owns that file.

## Open questions
None. The strengths come from the vision's rule 7 and from what each skill's `SKILL.md` says it runs.
