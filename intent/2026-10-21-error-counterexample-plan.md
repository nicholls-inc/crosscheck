# Plan: Each bound, choices and nullability error carries a proved counterexample

Intent: `intent/2026-10-21-error-counterexample.md`. Spec: `intent/2026-10-07-error-witness-spec.md`. Task CG-1.35, issue #8, roadmap items CG-1 and TB-1. Tier 3: the review of #133 added `runCheckerCx_fst` to the protected theorems, so the rules file, the generator and the manifest change.

## Files, in order

1. `cgv/prover/ContractGraph/Counterexample.lean`. `Counterexample`, `admits`, `counterexampleCandidate`, `counterexampleOf` with `counterexampleOf_sound`, `checkConstraintPair_counterexample`, and the Python text (`toPython`). A choice carries the target field's value type, and `toPython` writes an integer choice of an `"int"` field as an int literal (WT-6).
2. `cgv/prover/ContractGraph/Main.lean`. `runCheckerCx` and the paired reporting functions, `runCheckerCx_fst`, the `"counterexample"` key in `outputToJson`, and `main` calling `runCheckerCx`.
3. `cgv/src/model_extractor.rs`, `cgv/src/dataclass_extractor.rs` and `cgv/prover/ContractGraph/Translation.lean`. The choices contract records the value type of its field, and translation copies it to the constraint's `typeName`.
4. `cgv/src/report.rs` and `cgv/scripts/check-fixtures.sh`. The text line of WT-9 and the fixture check of WT-11.
5. Tests. `cgv/prover/ContractGraphTest/Counterexample.lean`, `cgv/tests/e2e_counterexample.rs`, and `expected.json` of `plain_python`, `r3_choices`, `r3_float_bounds` and `r5_choices_comma`.
6. `.assurance/protected-surface-amend/protect-runcheckercx-fst-2026-10-21.md`. The governance note, before the protected edits.
7. `.claude/rules/protected-surfaces.md` and `cgv/prover/scripts/ProtectedStatements.lean`. `runCheckerCx_fst` in the table and in `protectedTheorems`. Regenerate `cgv/prover/protected-statements.txt`.
8. Docs. `cgv/README.md`, `cgv/CLAUDE.md`, the spec (WT-6, WT-9, WT-12), the intent, `docs/TASKS.md` and the root `JOURNAL.md`.

## Risks

- **The paired output drifts from `runChecker`.** `runCheckerCx_fst` proves it does not, and the manifest now records that statement and the definitions it reaches.
- **A choice written in the wrong type.** The `#guard`s pin the int, str and unknown cases, and `r5_choices_comma` pins `3` end to end.
- **Manifest drift.** CGV CI regenerates the manifest and diffs it.

## Proof that it works

- `lake build` passes with no `sorry`.
- `lake env lean --run scripts/ProtectedStatements.lean | diff -u protected-statements.txt -` prints nothing.
- `scripts/manifest-selftest.sh` and `scripts/compiled-code-selftest.sh` exit 0.
- `cargo test` passes, and `scripts/check-fixtures.sh` passes every fixture. With the old rendering, `r5_choices_comma` fails.
