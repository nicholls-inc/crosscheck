# Evaluation on a real Django codebase (September 2026)

This report records the first measurement of the tool on a production codebase. It covers how many reported findings are real bugs, whether historical bugs would have been caught, and how the tool compares with type checkers and other approaches. The codebase is private, so it is described here by size only, and its bugs are described by shape rather than by file, model or commit.

Follow-up work is tracked in issues #4 to #11.

## Setup

- Codebase: a private Django application of about 12,000 functions. With `--exclude '**/tests/**' --exclude '**/migrations/**' --allow-parse-errors`, the extractor found about 3,000 model fields, 3,000 data class fields and 55,000 edges.
- Tool: the `main` branch after PR #3, built from source. A whole-repo run took 14 seconds.
- Result: exit 1, about 37,000 hop states checked, 237 errors and 8,374 warnings.

| Severity | Constraint kind | Count |
| --- | --- | --- |
| error | non-null | 190 |
| error | type | 44 |
| error | length | 2 |
| error | precision | 1 |
| warning | non-null | 7,595 |
| warning | length | 390 |
| warning | choices | 182 |
| warning | precision | 114 |
| warning | range | 93 |

## Question 1: how many errors are real bugs?

67 errors were triaged by reading the code at the site, at the origin of the value and at the callers. The sample was 20 non-null errors, one per distinct target, chosen at random with a fixed seed, and all 47 type, length and precision errors.

| Class | Non-null (20) | Type, length, precision (47) | Total |
| --- | --- | --- | --- |
| Reachable bug | 1 | 2 | 3 (4%) |
| Contract violated, no caller harmed | 4 | 7 | 11 (16%) |
| False positive | 15 | 38 | 53 (79%) |

Triage took about 1.5 file reads per finding. Scaled to all 237 errors, the sample suggests roughly 10 reachable bugs.

The three reachable bugs:

1. **Length, identifier field.** A 200-character identifier field on one model is written into a 17-character field on another. On Postgres a longer value raises `DataError`, and the write fails.
2. **Length, address field.** A 255-character address field is written into a 100-character field during a request. A long address gives a 500 response.
3. **Nullability, `.first()` under `type: ignore`.** A function annotated to return a model instance returns `QuerySet.first()`, with `# type: ignore[return-value]` on the line. Its only caller updates other records and then assigns to an attribute of the result, which raises `AttributeError` when no row matches.

The first two are invisible to mypy and pyright, which do not model `max_length`. The third was visible to mypy and silenced.

The false positives come from a small number of causes (issue #5):

| Cause | Count in sample |
| --- | --- |
| int passed where `float` is annotated (PEP 484 accepts it) | 35 |
| Parameter typed `object` treated as non-null | 4 |
| No narrowing after reassignment and early return, caller attribute guards, `in {literals}`, `k in d` | 4 |
| pydantic `model_validate(dict)` treated as a typed write | 4 |
| `assert_never` and exhaustive `match` treated as falling through | 2 |
| ORM invariants (`.first()` after `MultipleObjectsReturned`, `filter(f__gte=...)` excluding NULL) | 2 |
| Guards that raise in the callee or caller | 2 |

## Question 2: what do the warnings mean?

A non-null warning comes from `missingPostconditionWarning` in `Diagnostics.lean`. The source has no nullability guarantee of the kind the target requires. It does not mean the tool found a None.

- The 7,595 non-null warnings fall on 6,923 distinct lines and 4,883 distinct targets. Deduplicating by site and target leaves 7,590.
- Targets: 42% return annotations, 31% model fields, 27% parameters.
- In a random sample of 8, 3 could plausibly receive None, 1 could not, and 4 could not be decided without more context.

As a list for people to read, the warnings are unusable (issue #9). They still carry signal: see question 3.

## Question 3: would the tool have caught past bugs?

This is a retrospective replay. About 14 months of history were searched for fixes in the tool's bug classes. Each candidate was classed in scope or out of scope **before** running the tool. In scope means a None, decimal place, length, range, choice or type mismatch reaching a Django field, data class field, return annotation or parameter on such a path. The tool was then run on the fix commit and on its parent.

| Bug shape | In scope | Result |
| --- | --- | --- |
| Optional address fields passed to non-Optional parameters that feed a non-null field | yes | **Caught.** 3 errors on the exact lines, all gone after the fix. mypy had flagged the same lines, which were silenced. |
| Nullable datetime written to a non-null `DateTimeField`; a production `IntegrityError` | yes | **Detected, not cleared.** Error on the exact line. The fix guarded the calling task and left the write unguarded, so the error remains. |
| Nullable timestamp two attributes deep written to a non-null field | yes | Warning only |
| `dict.get(k, "")` returning None for a null value, into a non-Optional data class field | yes | Warning only |
| Nullable attributes two levels deep returned under `-> str` (open tech debt) | yes | Warning only |
| Admin form saving a NULL foreign key | no | Nothing reported |
| External API returning `null` into a pydantic `str` field (2 cases) | no | Nothing reported |
| `str(None)` sent to an external API | no | Nothing reported |
| Crashes from using a None value (2 cases) | no | Nothing reported |
| Float-to-Decimal currency imprecision not reaching a `decimal_places` field | no | Nothing reported |
| Unique-constraint race | no | Nothing reported |

Summary: of 5 in-scope bugs, 2 were flagged as errors on the exact line, 1 of which the fix cleared. The other 3 were warnings. None was missed outright. None of the 8 out-of-scope bugs produced an error.

History held no replayable fixes for precision, length or choices. Every such change was a feature, or it was squashed into a single commit with no pre-fix tree.

**Rank of the real bug in the pre-fix run.** The rank is the position in file and line order among all errors or warnings:

| Case | Errors in run | Warnings in run | Rank of real finding |
| --- | --- | --- | --- |
| Optional address fields | 241 | 7,285 | errors #140 to #142 |
| Production `IntegrityError` | 251 | not recorded | error #176 |
| Timestamp two attributes deep | 219 | 7,746 | warning #2,397 |
| `dict.get` default | 233 | 8,355 | warning #3,094 |

Why three bugs were warnings (issue #6):

- attribute reads two levels deep are not resolved;
- `dict.get(k, default)` is treated as unknown rather than possibly None.

## Question 4: how does it compare with other tools?

mypy 2.3.1 (strict, django-stubs 6.1.1, pydantic plugin) and pyright 1.1.414 (strict) were run on four fixtures and two probe files.

| Bug | This tool | mypy | pyright |
| --- | --- | --- | --- |
| 6dp written into `decimal_places=3` (`bug1`) | error | nothing | nothing |
| Precision error visible only across 2 hops (`transitive`) | error | nothing | nothing |
| 4dp into 2dp (`nullable`) | error | nothing | nothing |
| `return None` under `-> Invoice` (`nullable`) | error | error | error |
| `Optional[int]` into a non-optional pydantic field (`plain_python`) | error | missed (plugin leaves `__init__` untyped) | error |
| String up to 64 characters into `max_length=32` (`plain_python`) | error | nothing | nothing |
| `Optional[Decimal]` into a non-null `DecimalField` via `create`, constructor and `save` | error | error | nothing (no Django plugin) |
| `Optional` coming through a `@property` or tuple unpacking | warning, exit 0 | error | error |

When the docstring `ensures: max(input_precision, 3)` was removed from `transitive`, the tool still inferred the dependent bound from the function body and reported the error.

**Where the tool is distinct.** It checks value-level constraints (length, decimal places, choices, range) against model field definitions, composed across several hops. Postgres rounds extra decimal places without an error and SQLite ignores `max_length`, so tests often miss these as well. Two of the three real bugs in question 1 are this kind.

**Where mypy is ahead.** mypy with django-stubs already covers most nullability into Django fields. It also covers Optionals through properties, tuples and comprehensions, and wrong base types. For nullability, this tool adds whole-graph coverage and places where mypy was silenced, not a replacement for mypy.

**Formal verification.** Nagini and CrossHair need hand-written contracts and a model of the Django ORM and `Decimal.quantize`, neither of which exists. That is weeks of work per module. This tool infers its contracts, so it is much cheaper and much weaker.

**An LLM reviewing call chains.** An LLM is better at intent, units (kWh against Wh) and business rules, none of which this tool models. This tool is better in the following ways:

- it enumerates every discovered data path;
- it gives the same answer on every run;
- it cannot invent a path or a field;
- it tags every fact with a file and line;
- it proves that no discovered path was skipped (`closedStates_checkPath`, `runChecker_sound_all`). A run that exceeds its budget exits 2 rather than passing.

**Public incidents.** Most well-known incidents in this area fall outside the tool's model:

- external payloads overflowing a `varchar`, for example dj-stripe issue 2038 and social-app-django issue 262;
- writes made by Django admin and forms;
- unit mismatches, such as the Mars Climate Orbiter;
- deployment mismatches, such as Knight Capital.

The case that fits is Django's `DecimalField` rounding on save without an error, which Django confirms as intended (Django ticket 24636).

## Claims that are not defensible today

- **"Exit 0 means every data path is consistent", stated without its qualifiers.** It holds only for paths the extractor discovered, and only against the translated constraints. Writes through admin, forms, DRF and raw SQL are not seen. A missing guarantee is a warning, not an error, so real None writes through a property or a tuple leave exit 0.
- **"The translation is proved relative to the behaviour model."** `docs/design/system-design.md` still says this. The README and CLAUDE.md say the translation has no theorems (issue #11).
- **A precision "inconsistency" is always a bug.** Rounding at storage is sometimes intended.
- **"Catches what type checkers miss", without the reverse.**
- **Any precision or recall figure, other than the ones in this report.**

## Recommendations

| Recommendation | Issue |
| --- | --- |
| A repeatable benchmark: replay cases and labelled findings, with hit rate, rank and precision tracked across versions | #4 |
| Remove the main false-positive causes | #5 |
| Close the extractor gaps that turn real bugs into warnings | #6 |
| A baseline mode that reports only what a change introduces | #7 |
| A checkable witness for each error: concrete value, later a failing test | #8 |
| Hide missing-guarantee warnings by default and report them as coverage | #9 |
| README scope statement and the qualifiers on exit 0 | #10 |
| Correct the trust model in `system-design.md` | #11 |

## Limitations of this evaluation

- The triage sample is 67 of 237 errors. The non-null sample is 20 of 190.
- Triage was done by reading code, not by executing it. Two findings kept a residual doubt.
- The replay covers about 14 months of history and 5 in-scope bugs, all nullability.
- The codebase cannot be named or shared, so these numbers cannot be reproduced from this repository. The benchmark in issue #4 is meant to make later evaluations reproducible, with private corpora kept outside the repo.
