# Spec: differential test of `BehaviorModel.lean`

Intent: `intent/2026-10-21-behavior-model-diff.md`. Governing roadmap item: TB-1. Task: TB-1.14. Issue: #16.

The executable form of this spec is `cgv/scripts/behavior_diff.py`, its baseline `cgv/scripts/behavior-baseline.json`, and its `selftest` subcommand. The results of the first run are in `cgv/docs/evaluation/behavior-model-diff-2026-10.md`.

- **BD-1. One evaluator, from the Lean text.** `cgv/prover/scripts/BehaviorEval.lean` imports `ContractGraph.BehaviorModel` and answers `true` or `false` for one JSON case per line. Each `Decidable` instance unfolds its predicate and infers the instance from the body, so editing a predicate changes the answer. The harness compiles the `BehaviorModel.lean` it is given with the toolchain in `cgv/prover/lean-toolchain`, and runs the evaluator with `lean --run`. It needs no `lake` and no `leansqlite`. An unknown predicate, a malformed argument or a missing answer stops the run with exit 2.
- **BD-2. Inputs.** Hypothesis draws the inputs for each configuration and predicate, from a fixed seed (default `20261021`) and a fixed example count (default 1000), with the example database off and only the generate phase. Hypothesis also mixes literals from local source files into its draws, and the harness turns that off, so editing `behavior_diff.py` does not change the inputs. The draws do not depend on any verdict, so the same seed and versions give the same inputs.
- **BD-3. Library verdicts.** For each input, the harness runs the real library and records whether it accepted the value.

  | Predicate | Library call | Lean arguments |
  | --- | --- | --- |
  | `decimalFieldAccepts` | `DecimalField(max_digits=m, decimal_places=d, blank=True).clean(v, None)` | `m, d`, and the digit counts of BD-4 |
  | `notNullAccepts` | `IntegerField(null=False, blank=True).clean(v, None)` | `v is None` |
  | `nullableAccepts` | `IntegerField(null=True, blank=True).clean(v, None)` | `v is None` |
  | `charFieldAccepts` | `CharField(max_length=n, blank=True).clean(s, None)` | `n, len(s)` (code points) |
  | `positiveIntFieldAccepts` | `PositiveIntegerField(blank=True).clean(v, None)` | `v` |
  | `minValueAccepts`, `maxValueAccepts` | `IntegerField(blank=True, validators=[MinValueValidator(l)])` (or `Max`), values in the 32-bit range | `l, v` |
  | `choicesAccepts` | `CharField(max_length=100, choices=..., blank=b).clean(v, None)`, with `b` drawn | `choices, v` |
  | `maxValueAcceptsMicros`, `minValueAcceptsMicros` | Django: `DecimalField(max_digits=30, decimal_places=6)` with `MaxValueValidator(l)` (or `Min`). pydantic: `Decimal` with `Field(le=l)` (or `ge`). `l` and `v` have at most 6 places | `l·10^6, v·10^6` |
  | `pydanticDecimalAccepts` | `Decimal` with `Field(max_digits=m, decimal_places=d)` | as `decimalFieldAccepts` |
  | `pydanticMaxLengthAccepts` | `str` with `Field(max_length=n)`, or `constr(max_length=n)` | `n, len(s)` |
  | `pydanticLeAccepts` | `int` with `Field(le=n)`, or `Field(lt=n)` | `n, v`, or `n - 1, v` |
  | `annotationAcceptsNull` | `x: int`, `Optional[int]`, `int \| None` or `Union[int, None]` | whether the annotation allows None, `v is None` |
  | `annotationAcceptsNumeric` | `x: T` under `ConfigDict(strict=True)`, for `T` and the value's type among `int`, `float`, `str`, `bool`, `Decimal` | the value's type name, `T` |

  The Django predicates run in four configurations: Django 4.2.30 and 5.2.18, each on the SQLite backend and on the dummy backend, whose operations carry Django's base integer ranges (the ranges PostgreSQL uses). The pydantic predicates run once, with pydantic 2.13.5.
- **BD-4. Digit counts.** For a finite `Decimal` as written, fractional digits are `max(0, -exponent)`, integer digits are the decimal digits of `int(abs(v))` (none for zero), and total digits are their sum. The Lean text does not define these counts. This is the reading the harness applies to it, and it does not copy either library's counting.
- **BD-5. Report.** For each configuration and predicate the report gives the inputs drawn, the distinct inputs, how many the library and the model accept, the disagreements in each direction (the library accepts and the model rejects, and the reverse), and the three shortest inputs in each direction. It goes to `cgv/target/behavior-diff/report.json` and, as a table, to standard output, with the seed and the package versions.
- **BD-6. Verdict.** The baseline holds, for each configuration and predicate, three counts: the inputs drawn, the inputs the library accepts and the model rejects, and the inputs the model accepts and the library rejects. With the baseline's seed and example count and every configuration, the run fails (exit 1) when any of the three counts differs from the baseline, in either direction of change, or when a configuration and predicate is missing from the run or from the baseline. With another seed, count or a subset of configurations, it fails when a configuration and predicate disagrees in a direction for which the baseline records no disagreement. In both cases it fails when a predicate disagrees and `tracked_by` in the baseline names no task row for it. Otherwise it exits 0. `--update-baseline` rewrites the counts and keeps `tracked_by`.
- **BD-7. Self-test.** `behavior_diff.py selftest` writes one mutant of `BehaviorModel.lean` per predicate family, each in its own scratch copy: `charFieldAccepts` with `<` for `≤` (char and length), the fractional-digit conjunct of `decimalFieldAccepts` with `<` for `≤` (decimal), `maxValueAccepts` with `<` for `≤` (integer range), `choicesAccepts` with `∉` for `∈` (choices), `nullableAccepts` rejecting null (nullability), and `pydanticLeAccepts` with `<` for `≤` (a pydantic predicate). It runs BD-1 to BD-6 against each, and passes only if every run exits 1 and the configurations and predicates whose counts moved are exactly the ones that evaluate the mutated predicate. The decimal mutant must move `pydanticDecimalAccepts` too, because that predicate is defined as `decimalFieldAccepts`. It prints each mutant as killed or survived. The committed `BehaviorModel.lean` is not touched.
- **BD-8. Pinned inputs.** `cgv/scripts/behavior-requirements-django42.txt` and `-django52.txt` pin every package with hashes, for Python 3.12. The harness installs each into its own virtual environment under `cgv/target/behavior-diff/` with `pip --require-hashes`, and reinstalls when the file or the interpreter changes.

**Known gaps, not rules.**
- A value with more than 6 decimal places for the micros predicates, a `D` other than 6, and the rounding of extractor micros are not yet reached. The blocking property is that the Lean text states the scaled comparison for integers only, so a value that does not scale to an integer has no reading. The open question is whether TB-1.16 should state how a value is scaled.
- `NaN`, infinities and floats are not drawn for the decimal predicates. The model says nothing about them.
- MySQL and Oracle integer ranges are not yet reached. The blocking property is that their backends need a client library and, for some behaviour, a server. The open question is whether TB-1.24 runs a database in CI.
- `Field(strict=True)` and the `Strict*` types are not drawn for `annotationAcceptsNumeric`, only `ConfigDict(strict=True)`. The extractor reads all three.
- A predicate that agrees on every drawn input is `tested` on that count, not proved. A disagreement on a rare input can be missed.
