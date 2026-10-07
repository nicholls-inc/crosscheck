# Plan: CGV accepts an `int` where a `float` is required

Intent: `intent/2026-10-06-cgv-numeric-tower.md`. Spec: `cgv/test_fixtures/numeric_tower/expected.json` and the changed Lean definitions below. Task: CG-1.7. Governing roadmap item: CG-1.

## Files that change, in order

1. `cgv/test_fixtures/numeric_tower/{ok.py,code.py,expected.json}`. The spec. `ok.py` holds the two reproduced false positives (`int` parameter into a `float` dataclass field, `int` argument into a `float` parameter) and `int` and `float` into a `complex` field. `code.py` holds three errors that must stay: `Decimal` into `float`, `str` into `float`, `float` into `int`. On `origin/main` the fixture fails with two unexpected errors.
2. `.assurance/protected-surface-amend/cgv-numeric-tower-2026-10-06.md`. The governance note, naming `cgv/prover/ContractGraph/BehaviorModel.lean` and `cgv/prover/protected-statements.txt`. It must be committed before either file is edited, because the PreToolUse hook blocks the edit otherwise.
3. `cgv/prover/ContractGraph/Checker.lean`.
   - A new `def typeAccepts (st tt : String) : Bool := st == tt || (st == "int" && tt == "float")`.
   - `checkTypeConsistency` tests `typeAccepts sourceType targetType`.
   - The `.type` case of `constraintImplies` becomes `typeAccepts st tt = true`.
   - Repair `pair_sound` if its `.type` case no longer closes with `simp_all [checkTypeConsistency]`. No theorem statement changes.
4. `cgv/prover/ContractGraphTest/`: `#guard` lines that `checkConstraintPair` is consistent for `int` into `float` and inconsistent for `float` into `int`, `bool` into `float` and `Decimal` into `float`.
5. `cgv/prover/ContractGraph/BehaviorModel.lean`: a rule `annotationAcceptsNumeric` stating that an annotation `float` accepts a value of type `int`, citing PEP 484 and the pydantic 2.11.10 run in the intent. Its comment says that `bool` into `float` and the `complex` half of the tower are not stated, and why.
6. `cgv/prover/protected-statements.txt`: regenerate with `lake env lean --run scripts/ProtectedStatements.lean`, write it with the Write tool, and check that the only differences are the hashes of `constraintImplies` and the new `typeAccepts` entry.
7. `cgv/CLAUDE.md` ("Constraint matching", "type checks equality") and `cgv/README.md` wherever it says the type check is equality: state the one widening.
8. `docs/TASKS.md`: set CG-1.7 to `done` with this intent as its record, and add the coordinator's ORM row under CG-1 with the next free CG-1.x ID.
9. `JOURNAL.md` entry at the repository root.

## Risks
- **The guarantee shrinks.** Exit 0 no longer says anything about an `int` reaching a `float` requirement. The intent's table shows every `float` requirement CGV extracts accepts an `int` at runtime or under a PEP 484 checker. A future extractor change that adds a `float` requirement rejecting `int` (none is known) would make the rule wrong; the `BehaviorModel.lean` comment names the condition.
- **Composition.** The source's type name is unchanged, so a value typed `int` that crosses a `float` parameter and reaches an `int` field downstream is checked against the parameter's own `float` guarantee and still errors. No change to `composeContracts`.
- **Diagnostics.** `missingPostconditionWarning` keys on the presence of a type name, not its value, so it is unaffected.

## Proof that it worked
- `lake build ContractGraph ContractGraph.Main` passes, including `pair_sound` and the new `#guard` lines.
- The manifest regenerates with no axiom failure, and the diff touches only `constraintImplies`'s reach.
- `scripts/check-fixtures.sh` passes on every fixture, `numeric_tower` included, and `cargo test` passes.
- Mutation: reverting `typeAccepts` to `st == tt` makes `numeric_tower` fail with the two `ok.py` errors, and widening it to accept any pair makes it fail with the three `code.py` errors missing.
