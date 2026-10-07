# Intent: pydantic `model_validate` is a validation boundary

Task: CG-1.10. Governing roadmap item: CG-1. Issue: #5.

## Problem statement
CGV treats `Model.model_validate({...})` as a typed write. `special_writes` in `cgv/src/edge_discovery.rs` turns each known entry of the dict into a `writes_to` edge to the field, so the entry is checked against the field's type, nullability, precision, length, range and choices. The pydantic v1 name `parse_obj` takes the same path.

That is the wrong contract for these methods. `model_validate` takes `obj: Any`, so a type checker accepts any argument. pydantic then validates the input. In lax mode it coerces (`"true"` to `True`, `"1"` to `1`), and it rejects input that fails a constraint (None for a non-Optional field, a 4 decimal place value for `decimal_places=2`) with a `ValidationError`. A caller that hands raw data to `model_validate` relies on that rejection on purpose. On the codebase that `cgv/docs/evaluation/real-codebase-evaluation-2026-09.md` measured, this produced 4 of the triaged false positives: 3 type errors and 1 non-null error.

A typed constructor call `Model(f=v)` is different. Its signature has the field types, a type checker rejects a mismatched argument, and CGV checks it relative to a type-correct program, as `BehaviorModel.lean` states for data classes. That stays as it is.

## Proposed outcome
At a call to `model_validate`, `model_validate_json`, `model_validate_strings`, `parse_obj` or `parse_raw` on a pydantic class, an entry for a field whose contract validation enforces is no longer a write. An entry for any other field stays a typed write, as before. The call still returns a non-null instance, as `value_analysis.rs` already says, and reads of the instance's fields keep the facts of the field annotation.

Validation enforces a field's contract unless one of these holds. Each was measured with pydantic 2.13.5, except the v1 rule. pydantic v1 was not installed to measure it, so that rule is conservative: it keeps checks that v1 may not need.
- The field has `decimal_places` or `max_digits`. pydantic counts digits after it drops trailing zeros, so `Field(decimal_places=2)` accepts and stores `Decimal("1.2300")`, and `Field(max_digits=3, decimal_places=1)` accepts `Decimal("10.000")`. CGV counts the digits as written.
- The annotation is `SkipValidation[T]`, or has a `PlainValidator` or `WrapValidator` in its `Annotated` metadata. Each of these stores `None` from `model_validate({"label": None})` for a `str` field. A `BeforeValidator` still validates its result and rejects `None`.
- The field has a `None` default under an annotation without None (`label: str = None`). pydantic v1 reads that as Optional, so `parse_obj({"label": None})` stores `None`.

Measured with pydantic 2.13.5 as enforced, so their entries stay boundaries: `max_length` and `gt` reject `model_validate` input that breaks them. Other constraint kinds are not measured here and rest on the existing "enforced by validation at construction" rule.

`ClassInfo.validate_checked` holds the names of these fields, and the validation call in `special_writes` writes only their entries.

These stay writes for every field:
- `Model(f=v)`, a typed constructor call, including a type mismatch such as `Model(enabled="true")`;
- `obj.model_copy(update={...})`, which does not validate, so an invalid entry is stored.

Only `model_validate` and `parse_obj` were writes before. `model_validate_json`, `model_validate_strings` and `parse_raw` produced no write edges, because their argument is a string or a dict CGV did not read. They are named in the change so that every validating entry point is handled in one place.

A new fixture, `cgv/test_fixtures/pydantic_validate_boundary/`, pins the behaviour. Its `ok.py` passes a str to a `bool` field, an Optional value to a non-null field, a 4 character string to a `max_length=3` field, and an Optional value to a field with a `BeforeValidator`, through `model_validate` and `parse_obj`, and expects no error. Each source in `ok.py` is its own function, so an error from `ok.py` has a path that `expected.json` does not list. Its `models.py` makes the same mistakes through a typed constructor and `model_copy`, and makes each mistake of the list above through `model_validate` or `parse_obj`, and expects each error. The fixture's `expected.json` is the spec.

`cgv/test_fixtures/r5_write_patterns/` keeps its error for `Inv.model_validate({"total": four(x)})`, since `total` has `decimal_places=2`.

## Effect on the guarantee
Exit 0 promises less about one kind of call. For a field whose contract validation enforces, it no longer says that the input to a validation call passes validation. It says nothing false about the stored value, because validation rejects a value that breaks that field's contract before it is stored. For the other fields, exit 0 promises what it did before.

Three limits remain, all not yet reached:
- A report of a validation call whose input always fails validation. The property that blocks it is that CGV's checks ask whether a value may break a constraint. They do not tell a value that may fail validation from one that must fail. The open question is whether a must-fail check belongs in CGV, or in a witness-based report like CG-1.6.
- The boundary for one kind of constraint at a time. An Optional value given to a `decimal_places` field through `model_validate` is still a non-null error, though validation rejects it, because the whole entry stays a write. The property that blocks it is that an edge from a producer carries all of the producer's guarantees, so one kind cannot be dropped for one edge. The open question is whether edges should carry a per-kind mask.
- The unvalidated markers are recognised by their literal names. A renamed import (`SkipValidation as SV`), an alias of an unvalidated annotation (`Skip = SkipValidation[str]`) and an `Annotated[T, AfterValidator(f)]` (whose result pydantic does not revalidate) keep the field's requirements and drop the write, so exit 0 does not check an entry into them. The property that blocks it is that the extractor reads names, not what they import. The open question is whether it should resolve imports and aliases before matching, or treat an unresolved annotation head as unvalidated (CG-1.43).

## Affected users and systems
- People who run `crosscheck-contracts contracts check` on code that validates raw data with pydantic. They lose false errors at each `model_validate` call into a field whose contract validation enforces.
- `special_writes` in `cgv/src/edge_discovery.rs`, `DataClassField` in `cgv/src/dataclass_extractor.rs`, `ClassInfo` in `cgv/src/resolve.rs`, `cgv/src/extractor.rs`, their unit tests and the new fixture. No Lean file changes, so `protected-statements.txt` stays the same.

## Constraints
- `BehaviorModel.lean` needs no new rule, and in particular none for lax coercion. It already says that pydantic v2 `BaseModel` fields "are enforced by validation at construction". This change relies on that only for the fields where the measurements above confirm it. Lax coercion changes which inputs pass validation, not what is stored, so the change does not need to model it.
- `pydanticDecimalAccepts` in `BehaviorModel.lean` says validation rejects a value with more fractional or total digits, "the same predicate as Django's DecimalField". The measurement above shows that pydantic drops trailing zeros first, so the rule is false for a value such as `Decimal("1.2300")`. This change does not rely on it, since `decimal_places` and `max_digits` fields keep their writes. Amending the rule is a protected-surface change, so it is a separate row.
- A `PlainValidator`, `WrapValidator` or `BeforeValidator` in `Annotated` metadata may replace the value, as the decorator forms that `transforming_validators` handles may (`mode="before"`, `"wrap"` and `"plain"`; a plain-mode `field_validator` stores `None` in a `str` field, measured with pydantic 2.13.5). The decorator forms clear the field's requirements, and the `Annotated` forms do not. That gap is the same for a constructor call, and it is a separate row.
- Validators that run after field validation (`field_validator(..., mode="after")`, `model_validator(mode="after")`) can store a value that breaks the annotation, since pydantic does not validate their result again. That gap is the same for a constructor call and for `model_validate`, and this change does not widen it.
- `Model.model_construct(f=v)` (v1 `Model.construct`) skips validation, so it stores an invalid value. CGV records no write for it today, so `model_construct(label=None)` into a non-null field reports nothing. That false negative is a separate row.
- The tier is 2: a behavioural change to the extractor, with no protected path.

## Open questions
None.
