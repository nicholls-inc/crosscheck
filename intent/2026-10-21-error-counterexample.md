# Intent: Each bound, choices and nullability error carries a proved counterexample

Task: CG-1.35. Governing roadmap item: CG-1. Issue: #8. Spec: `intent/2026-10-07-error-witness-spec.md` (rules WT-1 to WT-12). The split that created this row is `intent/2026-10-07-error-witness.md`.

## Problem statement
A CGV error says that a source guarantee, such as `length ≤ 64`, does not imply a target requirement, such as `length ≤ 32`. To decide whether the error is real, the reader has to work out by hand which value breaks the target. Nothing in the output names such a value, so the reader cannot run anything to check the claim.

## Proposed outcome
Each error of kind `length`, `precision`, `range`, `range_min`, `choices` or `nullability` carries a counterexample, as WT-1 to WT-12 state. The Lean checker computes it from the two constraints of the error. The checker's JSON gives it as `"counterexample"`, and `--format text` prints it as the last line of the error block. A Lean theorem with no `sorry` proves that, for every such error of `checkConstraintPair` outside the three cases of WT-1, the value is admitted by the source constraint and rejected by the target. Fixtures pin a counterexample for each kind.

The strength of the counterexample is proved, relative to the translated constraints. It is a value that the source bound admits and the target bound rejects. No library has run it. CG-1.37 runs it against Django and pydantic, and CG-1.39 asks whether the code can produce it. CG-1.40 item 1 puts that strength in the output.

## Affected users and systems
- Anyone who reads a CGV error and has to decide whether it is real.
- `cgv/prover/ContractGraph/Counterexample.lean` (new), `cgv/prover/ContractGraph/Main.lean` (new definitions, and `main` and the JSON printer), `cgv/src/report.rs`, `cgv/scripts/check-fixtures.sh`, four fixtures' `expected.json`, new Lean and Rust tests, `cgv/README.md` and `cgv/CLAUDE.md`.
- No file on CGV's protected surfaces changes, and `cgv/prover/protected-statements.txt` stays the same (WT-12). The change is behavioural, so it is Tier 2.

## Constraints
- WT-12: the statement manifest must not change. The intent of the split assumed that a defaulted `counterexample` field on `ResultEntry` would leave the manifest unchanged. It would not: the manifest records the type of the constructor `ResultEntry.mk`, and the value hash of `buildResultEntry` and of every reporting function that `runChecker` calls. So the counterexample cannot live in `ResultEntry` or be computed inside those functions.
- The design that keeps the manifest unchanged: `runCheckerCx` in `Main.lean` builds the same report as `runChecker` with each entry paired with the counterexample of the diagnostic it came from. It mirrors `collectEntries`, `guardedEntries`, `stateEntries`, `Dedupe.step` and `reportStatesWith` on pairs. The theorem `runCheckerCx_fst` proves that its output is exactly `runChecker`'s, so the soundness theorems about `runChecker` still describe what the binary prints, and the exit code is unchanged. A finding that deduplication merges keeps the counterexample of the entry that deduplication keeps (WT-8).
- `counterexampleOf` returns a value only after checking that the source admits it and the target rejects it (`counterexampleOf_sound`), so a counterexample in the output always has that property, whichever diagnostic it comes from. `checkConstraintPair_counterexample` gives the other direction that WT-10 asks for: every error of a WT-1 kind outside the three cases has one.
- Rule 1 of the vision: the guarantee rests on the Lean kernel, not on an LLM. The Python text is not covered by the theorem (WT-10), and `#guard` tests pin it for each kind.

## Decided without the maintainer
The maintainer granted full autonomy for this run on 2026-10-21. These calls were made with the most conservative option that keeps the claims honest.

- **Where the counterexample is computed.** A field on `ResultEntry` would change the manifest, which WT-12 forbids, so the counterexample is paired with each entry outside the protected reporting functions, and a theorem ties the paired output to `runChecker`'s (see Constraints). A different design that changes the manifest would need a governance note and Tier 3, and no roadmap item asks for that.
- **Indentation of the text line.** WT-9 writes the line as `  counterexample: <text>`. Every other detail line of an error block is indented by seven spaces, so the report prints `       counterexample: <text>`, aligned with them. WT-9 now says the line is indented like the block's other lines. The key text and its position as the last line are unchanged.
- **What "admits" means for a missing bound.** `Counterexample.admits` treats a constraint with no static bound (or no choices list) as admitting every value of its kind. That case never reaches a counterexample, because `checkConstraintPair` reports no error without both bounds.

## Open questions
None. The known gaps that WT-1 and the spec list (the three excluded cases, a value that satisfies every source constraint at once, a fractional value against an integral field, a rounded source bound) stay not yet reached, with the blocking property and the open question that the spec names. CG-1.37 and CG-1.40 hold the follow-ups.
