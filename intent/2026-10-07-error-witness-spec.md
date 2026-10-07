# Spec: Each bound, choices and nullability error carries a counterexample

Intent: `intent/2026-10-07-error-witness.md`. Governing roadmap item: CG-1. Task: CG-1.35.

An error's *source constraint* and *target constraint* are the two constraints the checker compared (`DiagnosticInfo.sourceConstraint` and `targetConstraint`). S is the source constraint's static bound and T the target's. For `range` and `range_min`, both are value × 10^D, where D is the constraint's `scale`.

- **WT-1. Which results.** A result carries a counterexample exactly when its status is `inconsistent`, its severity is `error`, and its two constraints have the kind `length`, `precision`, `range`, `range_min`, `choices` or `nullability`, except a `precision` error with S < 1, which WT-3 cannot write. A precision error has S > T, and the extractor writes no negative precision bound, so S is at least 1 on its output. A warning, an incomplete result and a `type` error carry none.
- **WT-2. Length.** The counterexample is a string of S characters, written in Python as `"x" * S`.
- **WT-3. Precision.** The counterexample is the number 10^-S, which has exactly S fractional digits and no integer digit, written as `Decimal("0.` followed by S - 1 zeros, `1")`. For S = 4 that is `Decimal("0.0001")`.
- **WT-4. Range.** The counterexample is S / 10^D. It is written as a Python integer literal when it is a whole number, and otherwise as `Decimal("<v>")`, where `<v>` is the decimal text `formatScaled` gives for S and D. A negative whole number is written with a leading `-`.
- **WT-5. Range minimum.** As WT-4, with the source's lower bound S.
- **WT-6. Choices.** The counterexample is the first choice, in the order of the source's list, that the target's list does not contain. It is written as a Python string literal in double quotes, with `\` written `\\`, `"` written `\"`, and every character below U+0020 and U+007F written `\xNN` with two lowercase hex digits.
- **WT-7. Nullability.** The counterexample is `None`.
- **WT-8. JSON.** A result that carries a counterexample has the key `"counterexample"`, whose value is the Python text of WT-2 to WT-7 as a JSON string. Every other result has no such key. The key `"witness"` and every other key keep their meaning, and the exit code does not change. Two results that deduplication merges have the same counterexample.
- **WT-9. Text.** `--format text` prints `  counterexample: <text>` as the last line of each error block that has one.
- **WT-10. Proof.** A Lean theorem in `cgv/prover/ContractGraph/` states that for every pair of constraints that `checkConstraintPair` reports as an error of a WT-1 kind, the computed counterexample satisfies the source constraint and does not satisfy the target constraint. "Satisfies" is a predicate on a value and a constraint, defined with the same comparison as `constraintImplies` for that kind: a length or a fractional digit count at most the bound, a scaled number at most (or, for `range_min`, at least) the bound, a choice in the list, and None only when the nullability bound is not 0. The theorem and every definition it uses depend on no axiom other than `propext`, `Classical.choice` and `Quot.sound`. The Python text of WT-2 to WT-7 is not covered by the theorem. Unit tests pin it for each kind.
- **WT-11. Fixtures.** An entry of a fixture's `expected.json` may give `"counterexample"`. `scripts/check-fixtures.sh` then requires the matching error to carry exactly that text. At least one fixture pins a counterexample for each WT-1 kind.
- **WT-12. Protected surfaces.** `cgv/prover/protected-statements.txt` does not change, and no file on CGV's protected surfaces changes.

**Known gaps, not rules.**
- A counterexample satisfies the one source constraint the error names, not every constraint on the source. A source with `max_length=255` and a `choices` list of short strings admits no 255-character value. A value that satisfies all of the source's constraints at once is not yet reached. The property that blocks it is that the checker compares one pair of constraints at a time. The open question is whether the checker should search the source's constraints jointly, or say per error that it did not.
- The theorem is about the translated constraints. Whether Django or pydantic rejects the value is tested by CG-1.37, and whether the code produces it by CG-1.39.
