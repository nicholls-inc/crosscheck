# Intent: Check the integer digits that `max_digits` allows

Task: TB-1.13. Governing roadmap item: TB-1. Issue: #16.
Plan: step 3 of `intent/2026-10-07-prove-extraction-plan.md`.
Spec: `intent/2026-10-07-max-digits-spec.md`.

## Problem statement
`decimalFieldAccepts` in `cgv/prover/ContractGraph/BehaviorModel.lean` has three conjuncts: at most `d` fractional digits, at most `m` total digits, and at most `m - d` integer digits. The checker compares only the first. The extractor writes `param_max_digits` on the precision row, and `readContracts` selects it, but `ContractRow` has no field for it, so translation drops it.

Measured on `main` at `4f0c1e0` with binaries built from it. `cgv/test_fixtures/tb_max_digits/` holds `DecimalField(max_digits=5, decimal_places=2)` and the pydantic `Field(max_digits=5, decimal_places=2)`. Its `bad.py` writes `Decimal("123456.78")` and `Decimal("-1000")` into the Django field and `Decimal("1000.5")` into the pydantic field. Each has 2 or fewer fractional digits and more than 3 integer digits, so Django's `DecimalValidator` and pydantic reject them. The checker exits 0 with no result. The extractor already writes an exact range guarantee for each of the three writes (`range ≤ 123456.78`, and so on), so nothing is missing on the source side.

## Proposed outcome
The extractor turns `max_digits` into a range requirement. For `max_digits=m` and `decimal_places=d` the bound is `±(10^(m-d) - 10^-d)`: `m` nines with `d` of them after the point, `±999.99` for 5 and 2. It is intersected with any validator bound on the field. Without a `decimal_places` the extractor can read, the bound is `±(10^m - 1)`. The three writes in `bad.py` are errors, and the writes of `999.99` and `-999.99` in `ok.py` are not.

**The fork the task row left open, decided.** The row offered two routes: a range requirement from the extractor (Tier 2), or a new conjunct in `constraintImplies` (Tier 3, protected). The range route is taken, for three reasons, each read from the code.

1. A new conjunct needs a guarantee to compare against. Constraints interact only within one `ConstraintKind`, and no source has an integer-digit guarantee: the extractor knows the magnitude of a value as a range bound, not as a digit count. The conjunct would need a new guarantee shape from the extractor, or a cross-kind comparison inside `constraintImplies`, which changes the shape of every protected statement that mentions it.
2. The range route is exact for the integer-digit conjunct. A value with at most `d` fractional digits has at most `m - d` integer digits if and only if its magnitude is at most `10^(m-d) - 10^-d`. With the precision requirement, which already checks the fractional conjunct, the total-digit conjunct follows, because total digits are integer digits plus fractional digits.
3. The range route reuses the range check, the scaled-integer comparison and the warning for a source with no range guarantee, all already in `constraintImplies`. No protected surface changes, so the change is Tier 2. TB-1.16 links the range check to `BehaviorModel.lean`, and TB-1.18 then states the precision theorem about `precision` and `range` together, as its row already anticipates.

The bound is sound for a requirement: every value Django or pydantic accepts lies inside it. The largest value with at most `m` digits, `d` of them fractional, is `m` nines with `d` after the point. Without a readable `d`, the largest accepted value is `m` whole nines, because a value with a fractional part has fewer integer digits. Django counts digits with `Decimal.as_tuple()` and pydantic after `normalize()`, and neither changes the magnitude.

## Affected users and systems
- Anyone who writes a `DecimalField` or a pydantic `max_digits` field. A write whose magnitude is out of range is now an error. A write whose magnitude the extractor cannot bound gets the range warning that a `MaxValueValidator` field already gets: on the fixtures, warnings rise on 37 of the 98 other fixtures, by 146 in all and always in pairs (one for each side of the range). No error and no exit code changes outside the new fixture.
- A read of such a field now carries the range as a guarantee, because `requirement_facts` reads the same bounds.
- `cgv/src/bounds.rs`, `cgv/src/model_extractor.rs`, `cgv/src/dataclass_extractor.rs`, `cgv/README.md`, the bench baseline (warnings on `precision-two-hop` go from 1 to 3; no outcome changes).

## Constraints
- No protected surface changes. `Translation.lean`, `constraintImplies` and `BehaviorModel.lean` stay as they are, so `protected-statements.txt` does not change.
- Every fixture under `cgv/test_fixtures/` keeps the errors and exit code in its `expected.json`, and the bench corpus keeps every outcome in `cgv/bench/baseline.json`.

## Open questions
None. Two cases are not yet reached, and the spec names each one's blocking property and open question: the total-digit conjunct of a `max_digits` field without a readable `decimal_places` (TB-1.33), and a `max_digits` above 38 or a `decimal_places` above 30, which `Dec` cannot hold.
