# Intent: The evidence record format

Task: ER-1.1. Governing roadmap item: ER-1.

## Problem statement
`docs/VISION.md` says that every change an AI makes ships with a record of evidence. The record lists each claim about the change, how strongly the claim is supported, the trusted base the claim depends on, and a way to rerun the check behind it. No such record exists today, and no document says what one contains.

- CGV writes JSON diagnostics and an exit code (`cgv/README.md`, "Output" and "Exit codes"). Exit 0 says that every data path is consistent with the declared contracts. The output does not say that this is a proof, which theorem proves it, that the proof rests on the Rust extractor, `Translation.lean` and `BehaviorModel.lean`, or which command reruns it.
- Crosscheck's pipelines prove Dafny code, check Lean models, and run differential random tests against a Lean oracle. Each prints its own result in its own shape. A reader cannot tell a proof from a sampled test from an LLM's opinion without reading the skill that produced it.
- Rule 7 of the vision says every claim names its strength (proved, tested, observed or judged), and that "verified" on its own is never a claim. Rule 3 says every result reruns from pinned inputs, and a sampled result records its seed. Nothing checks either rule, because there is no record to check.

ER-1.2, ER-1.3 and ER-1.4 each depend on this task. They need one format to emit and one format to check.

## Proposed outcome
- A spec, `intent/2026-10-06-evidence-record-spec.md`, defines version 1 of the evidence record: a JSON document that names the commit it describes and lists its claims.
- Each claim states what is claimed, its strength and the facts that strength needs, its trusted base, the command that reruns it with the exit code the rerun must reproduce, and the requirement path it traces to or an explicit `null`. The record does not show that a person approved the requirement; the spec flags that. The spec's tables give the fields.
- The spec lists the rules that the ER-1.4 checker applies, so that the checker rejects a record when a claim names no strength or no rerun command, as the roadmap's acceptance for ER-1 asks.
- The spec gives one worked record for CGV's exit 0 and one for a Crosscheck pipeline, so that ER-1.2 and ER-1.3 have a target.
- `docs/TASKS.md` marks ER-1.1 `done` with this file as its record.

## Affected users and systems
- Whoever decides whether to trust a change: an executive reads the claims and their strengths, an engineer reads the trusted base and reruns a command, and an auditor reruns every command and compares the results with the record.
- ER-1.2 (CGV emits a record), ER-1.3 (one Crosscheck pipeline emits a record) and ER-1.4 (a deterministic checker) build on the spec. This task changes no code in either tool.
- `docs/TASKS.md` and `JOURNAL.md`, for the row and the entry of this task.

## Constraints
- The format serves rules 3 and 7 of `docs/VISION.md`. It names four strengths and no others, so "verified" cannot appear as a strength.
- Rule 3 is only partly served. The format asks every claim for a rerun command, and a `tested` claim for its seed. Pinned reruns are not yet reached: the property that blocks them is a pinned, sandboxed environment for each command, and the open question is how to pin the toolchains. Reproducing an `observed` or `judged` claim is also not yet reached, because the observation or judgment lives outside the repository, with no pinned input that a command can regenerate. The spec flags both. It also flags four more gaps: a proof's axioms are not recorded, several independent checkers (rule 2) have no field, `BehaviorModel.lean` is listed in the CGV example though no theorem uses it, and the approval of a requirement is not recorded. A record also cannot sit in the commit it names, and CGV's per-contract levels do not map onto claim strengths; the spec flags both with their blocking property and open question.
- Rule 1 is asked for and not yet checked. The format makes a `judged` claim name its judge and gives an LLM's verdict no field, but no rule can tell a person's name from an LLM's. The property that blocks the check is that nothing in a record binds a judge to a person, and the open question is how a record shows that. Review is the only check today. The spec flags it.
- The checker that ER-1.4 adds must be deterministic and need no network and no LLM. So every rule in the spec is one a program can decide from the record alone. A rule that needs the world, such as whether a judge is a person or whether a rerun still passes, is flagged in the spec as a concern, not written as a checker rule.
- No new dependency. The format is plain JSON that Node, Rust and Lean read without a schema library.
- This task touches no protected surface. It writes two files under `intent/`, one row of `docs/TASKS.md` and one entry of `JOURNAL.md`.

## Flagged for the vision, not settled here
How evidence of different strengths combines into one verdict on a pull request is an open question of `docs/VISION.md`, and the roadmap asks this intent to flag it. The format does not answer it. Each claim keeps its own strength, and the record has no overall verdict field. That choice leaves the question open rather than settling it by accident, for example by treating a record whose claims are all `tested` as equal to one whose claims are all `proved`. ER-1's acceptance does not need a combined verdict, so the question does not block ER-1.2 to ER-1.4.

## Open questions
None that block the spec. The roadmap item states the fields a claim needs and the checker's two rejections, and the vision defines the four strengths. The combination of strengths, above, stays an open question of the vision. What an `observed` or `judged` claim reruns is a second open question, and it does not block the spec either. The spec flags it, and ER-1.2 to ER-1.4 do not need the answer. The spec's "Concerns flagged" also holds the questions of how to pin the toolchains and how a record shows that a test's inputs are fixed. They do not block either.
