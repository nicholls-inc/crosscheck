# Intent: Say what Layers 4 to 6 prove, test or only search in the research doc

Task: VA-1.7. Governing roadmap item: VA-1.

## Problem statement
VA-1.4 rewrote the Layer 4 to 6 strengths in `crosscheck/README.md` and `crosscheck/docs/assurance-hierarchy.md`. The research doc that the hierarchy guide links to for "the full treatment", `crosscheck/docs/research/assurance-hierarchy.md`, still says the old thing:

1. The Layer 5 section says "This layer is probabilistic" and quotes Claimcheck's ~96.3% accuracy as if it were the layer's strength.
2. The specification-chain summary says the chain "degrades from deterministic (Layer 4) to probabilistic (Layer 5, ~96%) to search only (Layer 6)".
3. The Layer 5 reachability paragraph says Layer 5 is "reachable as a probabilistic check" and tells the reader to "expect ~96% accuracy".
4. The Layer 4 section says "This is still deterministic", and its reachability paragraph says the coverage gate makes "every documented invariant" have "a covering test". The gate checks that a test file comments the invariant's ID, not that the test checks it.
5. The Layer 6 section says "claimcheck validates intent alignment", and the framing section says `/intent-check` and the other LLM tools "cover" surfaces formal methods do not reach.

Rule 1 of `docs/VISION.md` makes `/intent-check` a search tool whose output is never evidence. Rule 7 asks every claim to name its strength: proved, tested, observed or judged.

## Proposed outcome
The research doc agrees with the hierarchy guide:

- **Layer 4.** Where a verifier runs, it proves the code satisfies the spec as the spec now stands. A person confirms that an edited spec did not weaken. `/check-regressions` re-proves the Dafny source of each changed hard-constraint spec, not the extracted code, and for soft constraints only checks that the test file exists. Covering tests test, on the inputs they run. `/assurance-probe`'s mutation probe is deterministic mutation testing of the covering tests. The `/invariant-coverage-scaffold` gate checks a comment link, not that the test checks the invariant. `/rationale` proves and tests nothing itself.
- **Layer 5.** Search only for spec–intent alignment. The ~96% is the accuracy Claimcheck reports for the round-trip method on a development benchmark. It measures how often the search is right, not how well a spec matches intent, and `/intent-check`'s accuracy on real pull requests is unmeasured. An approved `/acceptance-oracle-draft` scenario that CI runs tests the code. A proof that a spec achieves its intent is not yet reached (RQ-1).
- **Layer 6.** Search only, as the doc already says. The within-module and cross-module passes of `/audit-invariant-consistency` are named here, as in the guide.

The grep `probabilistic \(Layer 5|layer is probabilistic|probabilistic check|Expect ~96%|still deterministic|validates intent` finds nothing in the file.

## Affected users and systems
- Anyone who follows the hierarchy guide's link to the research doc to decide how far to trust a Layer 4, 5 or 6 result.
- `crosscheck/docs/research/assurance-hierarchy.md`, `crosscheck/JOURNAL.md` and `docs/TASKS.md`. None is protected, and no behaviour changes, so the change is Tier 1.

## Constraints
- VA-1.6 already reworded the file's "out of scope", "not addressed" and "best-effort" text. Leave it as it is.
- No skill, agent or code changes. The skill text that still overstates strengths belongs to VA-1.5 and VA-1.11.
- The "Probabilistic complements" bullet in the framing section describes property-based testing and DRT, which test on sampled inputs. It is not a Layer 5 claim, so it stays.
- The implementation-chain summary says Layers 1 to 3 are "deterministically verifiable", while Layers 2 and 3 are not yet reached. That is outside Layers 4 to 6, so a new row, VA-1.13, owns it.
- Review of this PR raised VA-1.14: the placement of `/acceptance-oracle-draft`, its "exhaustive assurance" wording, and the fact that a person's confirmation that a spec did not weaken is a one-time check in the run where the spec changed. It is a new row, not part of this change.

## Open questions
None. The strengths come from rule 7 of the vision, from what each skill's `SKILL.md` says it runs, and from the wording VA-1.4 settled in the hierarchy guide.
