# Intent: pydantic `model_validate` is a validation boundary

Task: CG-1.10. Governing roadmap item: CG-1. Issue: #5.

## Problem statement
CGV treats `Model.model_validate({...})` as a typed write. `special_writes` in `cgv/src/edge_discovery.rs` turns each known entry of the dict into a `writes_to` edge to the field, so the entry is checked against the field's type, nullability, precision, length, range and choices. The pydantic v1 name `parse_obj` takes the same path.

That is the wrong contract for these methods. `model_validate` takes `obj: Any`, so a type checker accepts any argument. pydantic then validates the input. In lax mode it coerces (`"true"` to `True`, `"1"` to `1`), and it rejects input that fails a constraint (None for a non-Optional field, a 4 decimal place value for `decimal_places=2`) with a `ValidationError`. A caller that hands raw data to `model_validate` relies on that rejection on purpose. On the codebase that `cgv/docs/evaluation/real-codebase-evaluation-2026-09.md` measured, this produced 4 of the triaged false positives: 3 type errors and 1 non-null error.

A typed constructor call `Model(f=v)` is different. Its signature has the field types, a type checker rejects a mismatched argument, and CGV checks it relative to a type-correct program, as `BehaviorModel.lean` states for data classes. That stays as it is.

## Proposed outcome
A call to `model_validate`, `model_validate_json`, `model_validate_strings`, `parse_obj` or `parse_raw` on a pydantic class emits no write edges to the class's fields, whatever its arguments. The call still returns a non-null instance, as `value_analysis.rs` already says. Reads of the instance's fields keep the facts of the field annotation.

These stay writes:
- `Model(f=v)`, a typed constructor call, including a type mismatch such as `Model(enabled="true")`;
- `obj.model_copy(update={...})`, which does not validate, so an invalid entry is stored.

A new fixture, `cgv/test_fixtures/pydantic_validate_boundary/`, pins the behaviour. Its `ok.py` passes a str to a `bool` field, an Optional value to a non-null field, and a 4 decimal place value to a `decimal_places=2` field, through `model_validate`, `model_validate_json` and `parse_obj`, and expects no error. Its `models.py` makes the same three mistakes through a typed constructor and `model_copy` and expects each error. Before the change, `ok.py` produced 6 errors. The fixture's `expected.json` is the spec.

`cgv/test_fixtures/r5_write_patterns/` expects an error from `Inv.model_validate({"total": four(x)})`, a 4 decimal place value into a 2 decimal place field. That case becomes SAFE, and its expected error is removed.

## Effect on the guarantee
Exit 0 promises less about one kind of call. It no longer says that the input to a `model_validate` call passes validation. It still says nothing false about stored values, because validation rejects a value that breaks a field's constraints before it is stored. Exit 0 said nothing about `ValidationError` for input CGV could not see, such as the JSON string given to `model_validate_json`, before this change either.

A report of a validation call whose input always fails validation, such as the `r5_write_patterns` case, is not yet reached. The property that blocks it is that CGV's checks ask whether a value may break a constraint. They do not tell a value that may fail validation from one that must fail. The open question is whether a must-fail check belongs in CGV, or in a witness-based report like CG-1.6.

## Affected users and systems
- People who run `crosscheck-contracts contracts check` on code that validates raw data with pydantic. They lose false errors at each `model_validate` call.
- `special_writes` in `cgv/src/edge_discovery.rs`, its unit tests, the new fixture and `r5_write_patterns`. No Lean file changes, so `protected-statements.txt` stays the same.

## Constraints
- `BehaviorModel.lean` needs no new rule, and in particular none for lax coercion. It already says that pydantic v2 `BaseModel` fields "are enforced by validation at construction". That is the rule this change relies on: a value stored by `model_validate` meets the field's constraints. Lax coercion changes which inputs pass validation, not what is stored. This change makes no claim about inputs, so it does not need to model coercion.
- Validators that run after field validation (`field_validator(..., mode="after")`, `model_validator(mode="after")`) can store a value that breaks the annotation, since pydantic does not validate their result again. That gap is the same for a constructor call and for `model_validate`, and this change does not widen it.
- `Model.model_construct(f=v)` (v1 `Model.construct`) skips validation, so it stores an invalid value. CGV records no write for it today, so `model_construct(label=None)` into a non-null field reports nothing. That false negative is a separate row.
- The tier is 2: a behavioural change to the extractor, with no protected path.

## Open questions
None.
