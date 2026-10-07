# Spec: `max_digits` is a range requirement

Intent: `intent/2026-10-07-max-digits.md`. Governing roadmap item: TB-1. Task: TB-1.13.

The executable form of each rule is a test named after it: `test_digits_limit` and `test_narrow_to` in `cgv/src/bounds.rs`, `test_max_digits_bounds` in `cgv/src/model_extractor.rs`, `test_pydantic_max_digits_bounds` in `cgv/src/dataclass_extractor.rs`, and the fixture `cgv/test_fixtures/tb_max_digits/`.

- **MD-1. The limit.** `digits_limit(m, d)` is `m` nines with the last `d` after the point (`10^(m-d) - 10^-d`). With no `d` it is `m` whole nines (`10^m - 1`). It is `None` when `m < 1`, `d < 0`, `d > m`, `m > 38` (beyond `i128`) or `d > 30` (beyond `Dec`'s scale).
- **MD-2. Django.** A `DecimalField` with a literal `max_digits` has a range requirement of `[-limit, limit]`, intersected with its `MinValueValidator` and `MaxValueValidator` bounds. No other field type gets one, even with a `max_digits` keyword.
- **MD-3. pydantic.** A data class field with a literal `max_digits` (in `Field(...)`, `Annotated[..., Field(...)]` or `condecimal(...)`) has the same range requirement, intersected with its `ge`, `gt`, `le` and `lt` bounds. A field whose requirements a transforming validator clears keeps none.
- **MD-4. Reads.** A read of such a field carries the range as a guarantee, as for any validator bound.
- **MD-5. Verdicts.** In `tb_max_digits`, the writes of `Decimal("123456.78")` and `Decimal("-1000")` into `Price.amount` and of `Decimal("1000.5")` into `Quote.amount` are errors, and the writes of `999.99` and `-999.99` are not. Every other fixture gives the errors and exit code in its `expected.json`, and the bench corpus gives the outcomes in `cgv/bench/baseline.json`.

**Known gaps, not rules.**
- The total-digit conjunct of a `max_digits` field without a readable `decimal_places` is not yet reached. pydantic's `Field(max_digits=5)` rejects `99998.5` (6 digits), and the range `±99999` admits it. The blocking property is that the extractor has no total-digit guarantee for a source: it knows a value's magnitude and its fractional digits, not its digit count. The open question is whether such a guarantee is worth deriving (TB-1.33). With a readable `decimal_places`, the precision and range requirements together are the whole `decimalFieldAccepts` predicate.
- A `max_digits` above 38 or a `decimal_places` above 30 gives no range requirement, so integer-digit overflow of such a field is not yet reached. The blocking property is the size of `Dec` (an `i128` mantissa, scale at most 30). The open question is whether any real schema needs a wider bound.
- A write of a value whose magnitude the extractor cannot bound gives the range warning (`range (unspecified)`), not an error, as for every range requirement.
- No theorem links the range requirement to `decimalFieldAccepts` yet. TB-1.16 proves the range kind against `BehaviorModel.lean`, and TB-1.18 proves the precision kind together with it.
