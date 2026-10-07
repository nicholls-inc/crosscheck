# Intent: A parameter typed `object` accepts None

Task: CG-1.8. Governing roadmap item: CG-1. Issue: #5.

## Problem statement
CGV reports an error when a caller passes a value that may be None to a parameter annotated `object`. `None` is an instance of `object`, so a type checker accepts that call, and Python runs it. The error is a false positive. On the codebase that `cgv/docs/evaluation/real-codebase-evaluation-2026-09.md` measured, this cause produced 4 of the 20 triaged non-null errors. The labelled benchmark corpus has one case of it: `remember_nickname` in `cgv/bench/corpus/labelled/app/false_positives.py`, labelled `object-param`.

The cause is in `cgv/src/function_extractor.rs`. `param_info` and `apply_aliases` take a parameter's nullability from `annotation_facts` in `cgv/src/dataclass_extractor.rs`. That function starts every annotation at `Some(false)` and changes it only for `None`, `Optional`, a union with `None`, `Literal[None]` and `Any`. So `value: object` becomes a `non-null` precondition. Return annotations already treat `object` as allowing None: `annotation_nullability` gives it no contract.

Two related annotations already behave the right way, and no fixture pins them. `value: Any` gets unknown nullability, and an unannotated parameter gets none. Each gives no precondition, so a caller's Optional value is not an error.

## Proposed outcome
A parameter whose annotation names the type `object` gets the nullability `Some(true)`, the same as `Optional[object]`. So does a parameter annotated with a union that has an `object` member (`Union[object, int]`, `object | int`, quoted or inside `Annotated`), because that union is `object`. The union rule is on the parameter path only. `annotation_facts` and its union walk are unchanged, so fields keep their behaviour until CG-1.12. It has no `non-null` precondition, so passing an Optional value is not an error. Inside the function, the parameter's value may be None, so a write of it into a non-null field is an error, as it is for an `Optional` parameter.

A new fixture, `cgv/test_fixtures/object_param_none/`, pins the behaviour. Its `ok.py` passes an Optional value to parameters typed `object`, `Union[object, int]`, `object | int`, `Any` and nothing, and through each to a non-null field via `str(...)`, so no error is expected. Its `models.py` has one case where an `object` parameter is written into a non-null field. That is a real error, and the fixture expects it. The fixture's `expected.json` is the spec.

The `object-param` case in the labelled benchmark stops being reported.

## Affected users and systems
- People who run `crosscheck-contracts contracts check` on code that annotates a parameter `object`. They see fewer false errors, and one class of real error (an `object` parameter written into a non-null field) that was hidden before.
- `cgv/src/function_extractor.rs` and a new fixture. No Lean file changes, so `protected-statements.txt` stays the same.

## Constraints
- `BehaviorModel.lean` needs no new rule. Its rule `annotationAcceptsNull` covers the fields of plain-Python data classes and takes whether the annotation allows None as input. A function parameter's precondition rests on the type checker's semantics, which the README's trust model already names, not on that file.
- Data class fields typed `object` stay non-null in this change. `annotation_facts` serves fields too, and several fixtures (`r5_non_null`, `r5_generators`) use `x: object` as a non-null field with no type contract. The docstring of `annotationAcceptsNull` says that `x: T` with no `None` in the annotation accepts only non-None values, which is false when `T` is `object` or `Any`. Changing fields needs those fixtures reworked and that docstring amended, which is a protected-surface change. It becomes its own row, CG-1.12.
- The tier is 2: a behavioural change to the extractor, with no protected path.

## Open questions
None.
