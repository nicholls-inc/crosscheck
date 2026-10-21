# BehaviorModel.lean against Django and pydantic, 2026-10-21

Task TB-1.14, issue #16 (approach D for link 3). The claim is `tested`, with the counts below. It is not proved.

## How to rerun

From `cgv/`, with Python 3.12 and elan on the machine:

```bash
python3 scripts/behavior_diff.py run        # exit 0: every count equals scripts/behavior-baseline.json
python3 scripts/behavior_diff.py selftest   # exit 0: a mutant of charFieldAccepts makes the run fail
```

- Seed `20261021`, 1000 draws per configuration and predicate, 47,000 cases in all.
- Python 3.12.11, Django 4.2.30 and 5.2.18, pydantic 2.13.5 and Hypothesis 6.170.5, pinned with hashes in `scripts/behavior-requirements-django42.txt` and `-django52.txt`.
- Lean `v4.28.0` from `prover/lean-toolchain`. It compiles `prover/ContractGraph/BehaviorModel.lean` at `origin/main` `2efb07c` and evaluates each predicate with `prover/scripts/BehaviorEval.lean`.
- `sqlite` is Django's SQLite backend. `base` is Django's dummy backend, which carries the base integer ranges that PostgreSQL uses.

The run takes about 25 seconds once the virtual environments exist. Two runs on the same machine wrote byte-identical cases.

## Counts

Each cell is disagreements / draws (distinct inputs). An empty cell means the predicate has no library call in that configuration.

| Predicate | django42-sqlite | django42-base | django52-sqlite | django52-base | pydantic |
| --- | --- | --- | --- | --- | --- |
| `annotationAcceptsNull` |  |  |  |  | 0 / 1000 (527) |
| `annotationAcceptsNumeric` |  |  |  |  | 66 / 1000 (835) |
| `charFieldAccepts` | 0 / 1000 (1000) | 0 / 1000 (1000) | 0 / 1000 (1000) | 0 / 1000 (1000) |  |
| `choicesAccepts` | 125 / 1000 (965) | 125 / 1000 (965) | 125 / 1000 (965) | 125 / 1000 (965) |  |
| `decimalFieldAccepts` | 28 / 1000 (936) | 28 / 1000 (936) | 28 / 1000 (936) | 28 / 1000 (936) |  |
| `maxValueAccepts` | 0 / 1000 (983) | 0 / 1000 (983) | 0 / 1000 (983) | 0 / 1000 (983) |  |
| `maxValueAcceptsMicros` | 0 / 1000 (618) | 0 / 1000 (618) | 0 / 1000 (618) | 0 / 1000 (618) | 0 / 1000 (618) |
| `minValueAccepts` | 0 / 1000 (983) | 0 / 1000 (983) | 0 / 1000 (983) | 0 / 1000 (983) |  |
| `minValueAcceptsMicros` | 0 / 1000 (618) | 0 / 1000 (618) | 0 / 1000 (618) | 0 / 1000 (618) | 0 / 1000 (618) |
| `notNullAccepts` | 0 / 1000 (508) | 0 / 1000 (508) | 0 / 1000 (508) | 0 / 1000 (508) |  |
| `nullableAccepts` | 0 / 1000 (508) | 0 / 1000 (508) | 0 / 1000 (508) | 0 / 1000 (508) |  |
| `positiveIntFieldAccepts` | 500 / 1000 (997) | 103 / 1000 (997) | 9 / 1000 (997) | 103 / 1000 (997) |  |
| `pydanticDecimalAccepts` |  |  |  |  | 88 / 1000 (936) |
| `pydanticLeAccepts` |  |  |  |  | 0 / 1000 (990) |
| `pydanticMaxLengthAccepts` |  |  |  |  | 0 / 1000 (1000) |

Ten of the 15 predicates agree with the library on every draw. Five disagree. Every disagreement falls into one of the five findings below. A scratch script, not committed, sorted all 1,481 of them by the rule each finding states, and none was left over.

## Findings

"Model too wide" means the model accepts a value the library rejects. A requirement read from the model can then pass a write that fails at runtime. "Model too narrow" means the model rejects a value the library accepts, which can give a false error.

1. **Zero counts as a whole digit (TB-1.39).** `decimalFieldAccepts` and `pydanticDecimalAccepts`, model too wide, 28 per configuration. `Decimal("0")` (or `0E+2`) in a field with `max_digits == decimal_places`, such as `max_digits=2, decimal_places=2`. Django's `DecimalValidator` and pydantic count the zero as one integer digit and reject it. The model's text does not say how digits are counted. Under the plain reading the harness uses (zero has no integer digits), the model accepts it. `Decimal("0.00")` is accepted by both libraries and by the model. The `max_digits` spec of TB-1.13 already names this quirk for the extractor's range.
2. **pydantic drops trailing zeros (CG-1.17).** `pydanticDecimalAccepts`, model too narrow, 60 cases. pydantic accepts `Decimal("0.00")` for `max_digits=1, decimal_places=0`, because it counts digits after `normalize()`. CG-1.17 already records this, and this run confirms it.
3. **`PositiveIntegerField` bounds come from the backend (TB-1.40).** `positiveIntFieldAccepts` says "≥ 0" with no upper bound. On Django 4.2 with SQLite, the field accepts every negative value drawn (500, model too narrow), because SQLite's backend gives no integer range and the field has no validator of its own. With the base ranges, which PostgreSQL uses, both versions reject values above 2,147,483,647 (103, model too wide). Django 5.2 on SQLite rejects values above 2^63 - 1 (9, model too wide).
4. **An empty value skips the choices check (TB-1.41).** `choicesAccepts`, 125 per configuration, in both directions. Django checks choices only for a value outside `empty_values`, and `blank` decides `""`. With `blank=False`, `""` is rejected even when it is a listed choice (95, model too wide). With `blank=True`, `""` is accepted when it is not listed (30, model too narrow).
5. **Strict pydantic accepts `Decimal` for `float` (TB-1.42).** `annotationAcceptsNumeric`, model too narrow, 66 cases. Under `ConfigDict(strict=True)`, a `float` field accepts a `Decimal` and stores `float(v)`. The predicate allows only the same type or `int` into `float`. The model's prose claims strict pydantic as evidence for the `int` case, so the predicate is wrong for strict pydantic. It may still be right for PEP 484 checkers, which reject `Decimal` for `float`.

## What the run does not cover

The spec lists the gaps: values with more than 6 places for the micros predicates, `NaN` and infinities, MySQL and Oracle ranges, and `Field(strict=True)` and the `Strict*` types for the numeric predicate. A predicate with no disagreement is tested on the counts above, not proved, and a rare input can still disagree.
