# Intent: Say "not yet reached" in Crosscheck's README and hierarchy guide

Task: VA-1.2. Governing roadmap item: VA-1.

## Problem statement
`docs/VISION.md` says no class of code is outside the vision. When a tool does not reach a class, the docs must say "not yet reached", name the property that blocks it, and name the open question. It also says spec completeness is provable relative to a formal requirement.

Two of the first files a new user reads still state the old positions:

1. `crosscheck/README.md` calls performance degradation, networked failure modes and security "out of scope for the framework". It says Layers 2 and 3 are "deliberately not addressed". It calls `/spec-adversary` "best-effort probing" and Layer 6 "best-effort". Its section "What Crosscheck is not good for" calls itself an "explicit scope-limit".
2. `crosscheck/docs/assurance-hierarchy.md` says Layer 6 is "best-effort" in its TL;DR and in its table, and its Layer 2 row reads "Not addressed — trust your toolchain".

A reader who trusts these files concludes that Crosscheck has given up on these classes, which the vision says is false.

## Proposed outcome
Each of these statements says "not yet reached", names the blocking property and names the open question. The open questions come from the class table in `docs/VISION.md` where one fits. Layer 6 says that a proof of completeness is not yet reached because no formal requirement is tied to the spec, points to roadmap item RQ-1, and says `/spec-adversary` findings are search results, not evidence. The grep `out of scope|not addressed|best[- ]effort` finds nothing in either file. The Layer 3 row and the Byfuglien paragraph also say what CGV reaches and name its trusted base (the extractor, `Translation.lean`, `BehaviorModel.lean` and the soundness theorem statements), and name a blocking property and open question for Layers 2 and 3. Those two questions are not in the vision's class table, so this change proposes them.

## Affected users and systems
- Anyone deciding whether Crosscheck fits their code.
- `crosscheck/README.md`, `crosscheck/docs/assurance-hierarchy.md`, `crosscheck/JOURNAL.md` and `docs/TASKS.md`. None is protected, and no behaviour changes, so the change is Tier 1.

## Constraints
- No skill, agent or code changes. VA-1.1 owns the `intent-check` attestation wording in skills and agents, and VA-1.5 owns the "best-effort", "out of scope" and "not addressed" wording in them.
- The same phrases appear in `crosscheck/docs/research/assurance-hierarchy.md` and other docs under `crosscheck/docs/`. This change adds row VA-1.6 for them and does not edit them.
- Both files present `/intent-check`'s round-trip accuracy as Layer 5's confidence and say Layers 4 to 6 "prove" the spec is right. Rule 1 of the vision makes `/intent-check` a search tool. This change adds row VA-1.4 for that and does not edit it.

## Open questions
The vision names the wording and the open questions for the classes in its table. The Layer 2 blocking property and question (translation validation of each extracted file) and the Layer 3 ones (no shared contract format; composition checked end to end, not pair by pair) are not in that table. This change proposes them. The maintainer ratifies them by merging, or adds them to the vision's class table.
