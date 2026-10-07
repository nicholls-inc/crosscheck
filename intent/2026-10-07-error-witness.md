# Intent: Split CG-1.6 into one row per kind of checkable witness

Task: CG-1.6. Governing roadmap item: CG-1. Issue: #8.

## Problem statement
A user who does not yet trust CGV has to read code to decide whether each error is real. In `cgv/docs/evaluation/real-codebase-evaluation-2026-09.md`, triage read about 1.5 files per finding, and 3 of 67 triaged errors were reachable bugs. Handing that reading to an LLM moves the trust from the tool to the LLM, which rule 1 of `docs/VISION.md` rules out. Issue #8 asks for evidence that a person checks by running something: a concrete value that breaks the target constraint, the line and condition that produce None, and later a generated test that fails.

Row CG-1.6 asks one pull request for all of it. The work splits into four pieces of different strength, in different code:

1. **A counterexample value, proved.** For a bound, choices or nullability error, a value that the source guarantee admits and the target requirement rejects. The Lean checker already holds both constraints of every error (`DiagnosticInfo.sourceConstraint` and `targetConstraint` in `cgv/prover/ContractGraph/Types.lean`), so it can compute the value, and the Lean kernel can check a proof that the value is admitted and rejected. The proof is relative to the translated constraints. It says nothing about Django or pydantic, nor about whether the code produces the value.
2. **A test of the value against the real library, tested.** A generated pytest module that runs the target field's own validation on the value and expects a rejection, and, when the source is a field too, expects the source field to accept it. It checks the reading of `BehaviorModel.lean` for that one value, on a pinned library version, with no trust in CGV's semantics.
3. **The condition under which None is produced.** For a nullability error, the guard conditions that enclose the line that yields None, when the extractor knows them.
4. **A failing test of the code path, tested.** A generated test that calls the path's head with inputs that make the source produce the counterexample, and expects the sink to fail. Issue #8 says such a finding needs no trust in the tool. It is the only piece that shows the error is reachable.

The error kinds weigh differently than issue #8 suggests. Of the 237 errors in that evaluation, 190 were nullability, 44 were type, 2 were length and 1 was precision. Issue #8 says to start with the four numeric and string kinds. They hold 3 of the 237 errors, but 2 of the 3 reachable bugs, and their counterexample comes from data the checker already has. So the first row computes the value for those kinds, and for `choices` and `nullability`, where it is just as direct. Type errors get their own row, after CG-1.7 changes the type check.

## Proposed outcome
`docs/TASKS.md` replaces row CG-1.6 with five rows, in the order the evidence builds, and adds CG-1.40, which holds the hardening items of this pull request's own review:

- **CG-1.35.** The Lean checker computes a counterexample for each error of kind `length`, `precision`, `range`, `range_min`, `choices` and `nullability`, and proves it is admitted by the source constraint and rejected by the target. The checker's JSON carries it as `counterexample`, and `--format text` prints it. The rules are WT-1 to WT-12 in `intent/2026-10-07-error-witness-spec.md`.
- **CG-1.36.** The same for `type`, after CG-1.7. Its intent picks a value for each type name, since Django and pydantic convert some values (measured below).
- **CG-1.37.** `contracts check --witness-tests DIR` writes a pytest module per error whose target is a Django model field or a pydantic field, which checks the counterexample against the field's own validation. CGV CI runs the modules generated from the fixtures, with Django and pydantic pinned.
- **CG-1.38.** The extractor records the guard conditions that enclose a None-producing line, and the report prints them with the nullability error.
- **CG-1.39.** A generated test that drives the path from its head and expects the sink to fail, for the first shape the evaluation found real: a field of one model written into a field of another, starting with a head that takes the source instance as an argument.

No row names CG-1.6 in `Depends on`, so no dependency changes. This pull request also commits the spec of CG-1.35, so that the row's format is fixed before its code. Apart from that spec it does nothing else. It sets no row to `done`. Issue #8 stays open until the last of the five rows CG-1.35 to CG-1.39 is done.

## Affected users and systems
- Anyone who reads a CGV error and has to decide whether it is real.
- Later: `cgv/prover/ContractGraph/Types.lean`, `Diagnostics.lean` and `Main.lean` (CG-1.35, CG-1.36), `cgv/src/report.rs` and `cgv/scripts/check-fixtures.sh` (CG-1.35), a test generator in `cgv/src/` and the CGV CI workflow (CG-1.37), `cgv/src/flow.rs` and the contract database (CG-1.38).
- Now: `docs/TASKS.md`, this intent and the spec. None is protected, so the change is Tier 1.

## Constraints
- Rule 1 of the vision: a witness counts only when a deterministic run confirms it. An LLM may draft a test, but the test is evidence only once it runs and fails as stated. The rows ask for proofs and runs, not for an LLM's reading.
- Rule 7: each witness names its strength in the rows' prose, and CG-1.40 puts it in the output. The counterexample of CG-1.35 is proved relative to the translated constraints. The tests of CG-1.37 and CG-1.39 are tested, on one input each, against pinned library versions.
- The JSON key `witness` already names the whole data path of a result (`ResultEntry.witness` in `Types.lean`). The new field is `counterexample`, so no reader of the old key breaks.
- `DiagnosticInfo` is not in `cgv/prover/protected-statements.txt`, and `ResultEntry` appears there only in the statements of `runChecker_exitCode_eq_zero_iff` and `runCheckerPaths_exitCode_eq_zero_iff`, through its `severity` projection. No protected definition reaches either. So adding a defaulted `counterexample` field to `ResultEntry` should leave the manifest unchanged, as long as `severity` keeps its name and type. CG-1.35 should stay off CGV's protected surfaces. The row checks it.
- Task IDs are never reused. CG-1.35 to CG-1.40 appear in no row on `origin/main` (highest CG-1.32) and in no open pull request's diff (open pull request #105 uses CG-1.33 and CG-1.34), and no branch is named after them.

## Measured on Django 4.2.30 and pydantic 2.13.5
Run once by hand, and the probe is not committed. CG-1.37 reruns the checks in CI, on Django 4.2 and 5.x and on pydantic 2, and CG-1.40 adds the pydantic sources that were not measured. A data class source has no check that runs, so there is nothing to measure there. The property named in the next section for targets, that nothing which runs rejects a value outside an annotation, holds for a source too: nothing which runs accepts one.

A probe called `Field.clean(value, None)` on Django model fields and validated pydantic fields through `TypeAdapter`. It settles the value for each kind that CG-1.35 and the spec fix.

- `"x" * 255` is accepted by `CharField(max_length=255)` and rejected by `max_length=100` (code `max_length`). pydantic `Field(max_length=100)` rejects it (`string_too_long`).
- `Decimal("0.0001")` is accepted by `DecimalField(max_digits=10, decimal_places=4)` and rejected by `decimal_places=3` (`max_decimal_places`). With `max_digits=3, decimal_places=3` the code is `max_digits` instead, because Django checks the total digits first. So a generated test asserts the rejection, and records the code rather than requiring one.
- `10` is accepted by `IntegerField` with `MaxValueValidator(10)` and rejected with `MaxValueValidator(5)`. `Decimal("0.5")` is accepted by a `FloatField` with `MaxValueValidator(0.5)` and rejected with `0.2`.
- The extractor stores choice values as text, and an integer choice `1` as `"1"` (`literal_choice` in `cgv/src/resolve.rs`). The string `"3"` is accepted by `IntegerField(choices=[1, 2, 3])` and rejected by `choices=[1, 2]` (`invalid_choice`), because `to_python` converts it first. So the text of the choice, as a Python string, is a counterexample for both string and integer choices.
- `None` is rejected by a `CharField` without `null=True` (`null`) and accepted with `null=True, blank=True`. pydantic rejects `None` for `str` with `string_type`, not a code about null.
- `""` is rejected by a `CharField` without `blank=True` (`blank`). That is a constraint CGV does not model, and a counterexample must not rely on it.

## Not yet reached, with the blocking property and the open question
- **A library test for a target that Python does not enforce.** Data class, attrs (without validators), `NamedTuple` and `TypedDict` fields, function parameters and return annotations carry their contract in the annotation. The property that blocks a runtime test is that nothing that runs rejects a value outside an annotation. The open question is whether a type checker's verdict on a generated snippet should count as the target's rejection, given that the type checker joins the trusted base.
- **A failing path test for every error.** The property that blocks it is that CGV knows the value at the sink, not the arguments and database rows that make the head function produce it. The open question is whether to build those inputs from the source contract alone, which works when the head reads the source field off an argument, or to ask the user for fixtures. CG-1.39 takes the first shape and its intent settles the rest.

- **A counterexample for a `type` error.** This is CG-1.36. The property that blocks it today is that Django and pydantic convert some values before they validate them (`IntegerField` accepts the string `"3"`), so a value of the source type may not be rejected by the target for the reason the type names say. The open question is which value to give for each type name, and which type names have none that the checker can prove.

## Open questions
None for the split. Each new row carries its own question for its own intent. For CG-1.36, which value to give for each type name, given the conversions above. For CG-1.38, which guards the extractor can state as text a reader can check. For CG-1.39, how the inputs are built.
