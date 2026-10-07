# Intent: CGV accepts an `int` where a `float` is required

Task: CG-1.7. Governing roadmap item: CG-1. Issue: #5.

## Problem statement
CGV reports an error when an `int` reaches a place annotated `float`. On the private Django codebase that `cgv/docs/evaluation/real-codebase-evaluation-2026-09.md` measured, this caused 35 of the 53 triaged false positives, for example `def ma_to_a(ma: float)` called with an `int`. PEP 484 says that where an argument is annotated `float`, an `int` is acceptable, and that where it is annotated `complex`, an `int` or a `float` is acceptable. mypy and pyright accept these calls.

Reproduced on `origin/main` (9cb8972) with a two-function fixture: `def int_param_into_float(n: int) -> Reading: return Reading(amps=n, ...)` with `amps: float` on a dataclass, and `ma_to_a(n)` with `n: int`, give two errors, `type = int` against `type = float`. A literal `3` gives none, because literals carry no type guarantee. `int` and `float` into a `complex` field give none, because `complex` is not in `VALUE_TYPES` (`cgv/src/dataclass_extractor.rs`), so no `complex` requirement is ever extracted.

The `type` check is `st = tt` in `constraintImplies` (`cgv/prover/ContractGraph/Checker.lean`) and `sourceType == targetType` in `checkTypeConsistency`.

## Can the extractor fix this instead?
The task asks this first, because `constraintImplies` is a protected definition. It cannot, without a larger loss. A `type` constraint carries one type name and the checker compares names for equality, so the extractor has three moves, and each loses more than the checker change:

1. **Drop every `float` requirement**, as it already does for lax pydantic numeric fields (`DataClass::lax_numeric`). Then `Decimal`, `str` and `bool` into a `float` are no longer errors. The fixture below has a `Decimal` and a `str` case, and both are errors today.
2. **Rewrite an `int` guarantee to `float`.** Then an `int` into an `int` requirement becomes an error.
3. **Drop the `float` requirement on one edge when that edge's source is `int`.** The extractor sees one hop. The checker composes guarantees across hops, so an `int` that reaches a `float` requirement two hops later is still an error, and the rule would sit in untrusted Rust where no theorem and no manifest sees it.

Widening the check in Lean weakens the guarantee for exactly one pair, `int` into `float`, and the change shows in `protected-statements.txt`. That is the smaller weakening and the more visible one, so this change makes it.

## Proposed outcome
- `constraintImplies` and `checkTypeConsistency` accept a source type `int` against a target type `float`, as well as equal names. `pair_sound` and every theorem built on it still hold.
- `BehaviorModel.lean` gains the rule the check relies on: an annotation `float` accepts an `int`. It cites PEP 484 and the pydantic run below.
- `protected-statements.txt` is regenerated. Exit 0 promises less for one pair: an `int` written where `float` is required.
- A fixture `cgv/test_fixtures/numeric_tower/` has an `ok.py` with both false positives and the `complex` cases, which must stay error-free, and a `code.py` whose `Decimal` into `float`, `str` into `float` and `float` into `int` must stay errors.

## What the evidence says about the targets
A `float` requirement comes from a function parameter, a dataclass, attrs, `NamedTuple` or `TypedDict` field, or a strict pydantic field. Lax pydantic numeric fields and Django `FloatField` writes carry no `type` requirement. Measured with pydantic 2.11.10 on Python 3.12:

| Field | `1` (int) | `True` | `"1"` |
| --- | --- | --- | --- |
| `f: float` under `ConfigDict(strict=True)` | accepted, stored `1.0` | rejected | rejected |
| `StrictFloat`, `Field(strict=True)` on `float` | accepted | | |
| `c: complex` under `ConfigDict(strict=True)` | rejected | | |

So `int` into `float` holds for every target that carries a `float` requirement. The `complex` half of the tower does not hold for strict pydantic, which rejects both `int` and `float`.

## Affected users and systems
- CGV users: 35 of 53 triaged false positives on the measured codebase go away.
- `cgv/prover/ContractGraph/Checker.lean`, `BehaviorModel.lean`, `protected-statements.txt`, a new fixture, `cgv/README.md` and `cgv/CLAUDE.md` where they describe the `type` check. The change is Tier 3.

## Constraints
- No theorem statement changes. Only the value of `constraintImplies` and `checkTypeConsistency` changes.
- `bool` into `float` stays an error. PEP 484 type checkers accept it, since `bool` subclasses `int`, but strict pydantic rejects it (measured above). That is a new question, filed as #86, not settled here.
- `complex` stays without a `type` requirement. Accepting `int` or `float` into `complex` is already the behaviour, and the fixture pins it. Extracting a `complex` requirement is not yet reached: the blocking property is that strict pydantic rejects `int` and `float` into `complex`, so the check would need to know the field's strictness, and the open question is whether a strictness-aware `type` check belongs in the extractor or in `constraintImplies`.

## Open questions
- Where does a strictness-aware `type` check belong, in the extractor or in `constraintImplies`? It decides both `bool` into `float` (PEP 484 accepts, strict pydantic rejects) and the `complex` half of the tower. Not settled here; filed as #86.
- The evidence for "every extracted `float` requirement accepts an `int`" is the PEP 484 text and one pydantic 2.11.10 run. mypy, pyright and attrs validators were not run, so a new source of `float` requirements needs its own check before this rule is relied on for it. No issue tracks this yet; the next change that adds a source of `float` requirements must run mypy and pyright at pinned versions and record the versions in `BehaviorModel.lean`.
- The extractor reads annotation names syntactically, so a project that defines or imports its own `float` or `int` is read as the builtin, and the new `int` into `float` pair is accepted across that shadow. Name resolution is not yet reached; the premise is now stated in `BehaviorModel.lean`.
