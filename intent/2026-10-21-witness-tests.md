# Intent: Run each counterexample against the target field's own validation

Task: CG-1.37. Governing roadmap item: CG-1. Issue: #8. Spec: `intent/2026-10-21-witness-tests-spec.md` (rules WX-1 to WX-12). Plan: `intent/2026-10-21-witness-tests-plan.md`. The counterexample comes from CG-1.35 (`intent/2026-10-21-error-counterexample.md`, rules WT-1 to WT-12 in `intent/2026-10-07-error-witness-spec.md`).

## Problem statement
Since CG-1.35, a CGV error of kind `length`, `precision`, `range`, `range_min`, `choices` or `nullability` carries a counterexample in its JSON, such as `"x" * 12` for a 12-character source written into a 6-character field. The Lean kernel proves that the source bound admits the value and the target bound rejects it. That proof is about the translated constraints. It does not say what Django or pydantic do with the value, and `BehaviorModel.lean`, which states how they behave, is trusted, not proved. A reader who wants to see the target reject the value has to write the test by hand.

## Proposed outcome
`contracts check --witness-tests DIR` writes one pytest module per error whose target is a Django model field or a pydantic field and that carries a counterexample. Each module imports the user's own model class, takes the target field from it, and runs the field's own validation on the counterexample: `Model._meta.get_field(name).clean(value, None)` for Django, and `TypeAdapter` on the field's annotation and metadata for pydantic. The test asserts that the field accepts the value, which is what the code that writes it relies on. When the field rejects it, the test fails with a message that starts `CGV-WITNESS REJECTED` and names the library version, the field and the rejection codes. That failure is tested evidence, on this one value and on that library version, that the error's target rejects a value its source admits. A passing test means the library accepted the counterexample, so it is no evidence for the error.

A script, `cgv/scripts/witness_tests.py`, generates the modules for every fixture and runs them with Django 4.2 and 5.2 and pydantic 2 pinned, and checks that each test fails for the stated reason or is withheld for a named gap. A self-test replaces counterexamples with values the fields accept and checks that the script then fails. CGV CI runs both.

## Affected users and systems
- Anyone who reads a CGV error and wants to see the library reject the value before trusting it.
- `cgv/src/main.rs` (the flag), `cgv/src/witness.rs` and `cgv/src/witness_helper.py` (new), `cgv/src/db.rs` (a new table that records which library declared a field), `cgv/src/model_extractor.rs` and `cgv/src/dataclass_extractor.rs` (they fill it), new fixtures `cgv/test_fixtures/witness_tests/` and `cgv/test_fixtures/witness_tests_clean/`, `cgv/tests/e2e_witness_tests.rs`, `cgv/scripts/witness_tests.py` with two pinned requirements files, `.github/workflows/cgv-ci.yml`, `cgv/README.md`, `cgv/CLAUDE.md`, `docs/TASKS.md`.
- `.github/workflows/cgv-ci.yml` is a Class A protected surface, so this change is Tier 3 and carries the governance note `.assurance/protected-surface-amend/witness-tests-ci-2026-10-21.md`. No CGV proof surface changes: the Lean checker reads named columns of three tables and ignores the new one, so `BehaviorModel.lean` and `protected-statements.txt` stay as they are.

## Constraints
- Rule 1 of the vision: the evidence is a deterministic run of the library, never an LLM's reading. Rule 7: each module and each failure message names its strength (tested, on one value, on one library version) and says what it does not show (that the code produces the value, which is CG-1.39).
- The modules run offline. Installing the pinned libraries needs the package index once, and running the tests does not.
- `contracts check` prints the same stdout and exits with the same code with or without the flag, except that a failure to write the modules exits 2.
- Only the files that the generator wrote are removed from DIR on a later run, so a user's own files there survive.

## Measured on Django 4.2.30 and 5.2.18 and pydantic 2.13.5
CG-1.35's spec asked CG-1.37 to settle one gap first: what an integral target field does with a fractional counterexample such as `Decimal("5.5")`. The probe below ran `Field.clean(value, None)` on unbound Django fields and `TypeAdapter(Annotated[int, Field(...)])` in pydantic, in the pinned virtual environments of this task. The results are the same on both Django versions.

| Value | Field | Result |
| --- | --- | --- |
| `Decimal("5.5")` | Django `IntegerField` with `MaxValueValidator(5)` | accepted, cleaned to `5` |
| `Decimal("5.5")` | Django `IntegerField` with `MaxValueValidator(4)` | rejected, `max_value` |
| `Decimal("5.5")` | Django `PositiveIntegerField` | accepted, cleaned to `5` |
| `Decimal("-0.5")` | Django `PositiveIntegerField` | accepted, cleaned to `0` |
| `Decimal("5.5")`, `Decimal("-0.5")` | pydantic `int` with `le=5`, `le=4` or `ge=0` | rejected, `int_from_float`, whatever the bound |
| `Decimal("5.0")` | pydantic `int` with `le=4` | rejected, `less_than_equal` |
| `Decimal("0.7")` | pydantic `float` with `le=0.5` | rejected, `less_than_equal` |

Django's `IntegerField.to_python` calls `int()`, which truncates toward zero before the validators run, so a fractional counterexample can pass a bound it lies beyond. pydantic rejects every fractional value for an `int` field with `int_from_float`, inside the bound or not, so its rejection says nothing about the bound. Neither result is evidence about the bound the error names.

So the gap is settled as follows. For an integral target field the right counterexample is an integer: `⌊S⌋` for a `range` error and `⌈S⌉` for a `range_min` error, where S is the source bound. When that integer does not lie beyond the target bound, no value the field stores breaks it, and the error is about a value the field truncates (Django) or rejects for its type (pydantic). Computing that integer is a change to `Counterexample.lean`, which this task does not make. Until the checker does, a generated test whose counterexample is a fractional `Decimal` and whose target field is integral withholds both assertions and is skipped with the gap's text (WX-7). The integral check runs on the real field: `isinstance(field, models.IntegerField)` for Django, an annotation of `int` or `Optional[int]` for pydantic.

The probe, run with each pinned interpreter:

```python
from decimal import Decimal
from typing import Annotated
import django, pydantic
from django.conf import settings
settings.configure(USE_TZ=True)
django.setup()
from django.db import models
from django.core.exceptions import ValidationError
from django.core.validators import MaxValueValidator
from pydantic import TypeAdapter, Field

def dj(field, v):
    try:
        return f"accepted -> {field.clean(v, None)!r}"
    except ValidationError as e:
        return f"rejected {[x.code for x in e.error_list]}"

def pd(tp, v):
    try:
        return f"accepted -> {TypeAdapter(tp).validate_python(v)!r}"
    except pydantic.ValidationError as e:
        return f"rejected {[x['type'] for x in e.errors()]}"

for v in [Decimal("5.5"), Decimal("-0.5"), Decimal("5.0")]:
    print(v, dj(models.IntegerField(validators=[MaxValueValidator(5)]), v),
          dj(models.IntegerField(validators=[MaxValueValidator(4)]), v),
          dj(models.PositiveIntegerField(), v),
          pd(Annotated[int, Field(le=5)], v), pd(Annotated[int, Field(le=4)], v),
          pd(Annotated[int, Field(ge=0)], v))
print(pd(Annotated[float, Field(le=0.5)], Decimal("0.7")))
```

## Measured on the fixtures
`scripts/witness_tests.py run` on the 109 fixtures, with Django 4.2.30 and with Django 5.2.18, each with pydantic 2.13.5 and pytest 8.4.2: 76 fixtures produce witness modules, 183 modules in all. In both environments, 173 fail with `CGV-WITNESS REJECTED`, 2 are withheld (the fractional counterexamples of `witness_tests`) and 8 fail with the import errors of `IMPORT_GAPS`. No module passes. `witness_tests_clean` produces none. The self-test flags each of the 6 mutated modules of `witness_tests`, one per WT-1 kind, and no other, in both environments.

## Not yet reached, with the blocking property and the open question
- **The source-accepts assertion.** The row asks each test to expect the source field to accept the counterexample when the source is a Django or pydantic field. No error has such a source today: every edge leaves a function or call-site node (no edge of the databases of the 107 fixtures on `main` leaves a `model` node), and a guarantee read from a field, such as `instance.number`, is recorded on the function, without the field it came from. The generator emits the assertion whenever an error's source node is such a field, and withholds it for a `range` or `range_min` source until CG-1.40 item 5 says which bounds `scaledBound` rounded. The property that blocks it is that the contract database does not record which field a function's guarantee was read from. The open question is whether the extractor should record that field on the postcondition, or whether CG-1.39's path tests make the check unnecessary.
- **A fractional counterexample against an integral target field.** Settled above as to which value is right. Computing it is not yet reached. The property that blocks it is that `counterexampleOf` returns S itself for a range error, whatever the target field stores. The open question is only how to state `⌊S⌋` and `⌈S⌉` in `Counterexample.lean` and its proof, and the maintainer decides whether that becomes a task row.
- **pydantic validators beyond the annotation.** `TypeAdapter` on the field's annotation and metadata runs the constraints of `Field(...)`, `Annotated` metadata and constrained types. It does not run `@field_validator` or `@model_validator` methods, nor a model-wide `strict` setting other than the one the helper passes on. The property that blocks it is that those run only when the whole model validates, and building a whole valid model needs a value for every other field. The open question is whether to validate through `validate_assignment` on a constructed instance instead.
- **A target that Python does not enforce.** Data class, attrs, `NamedTuple` and `TypedDict` fields, parameters and return annotations get no module, as `intent/2026-10-07-error-witness.md` says, with its blocking property and open question. So does a pydantic field that the extractor marks `unvalidated`. The first run of the fixture modules found why: the three nullability errors on `SkipValidation[str]`, `PlainValidator` and `WrapValidator` fields of `pydantic_validate_boundary` passed, because pydantic accepted `None` there. Those errors rest on the annotation as a type checker reads it, like a data class field's.
- **A model module that cannot be imported on its own.** The test imports the user's module. With `DJANGO_SETTINGS_MODULE` set, it uses the user's settings. Without it, the helper configures minimal settings and registers the model's module as an app of its own, which covers a model with no relation to an installed app. The property that blocks the rest is that a module may need the user's settings, database or third-party packages to import. Two fixtures do: `r6_pydantic_forms` imports `pydantic_settings` and `sqlmodel`, and `r7_external_alias` imports `opentelemetry`, none of which the environments pin. The script lists them in `IMPORT_GAPS` and requires that exact import error, so the 8 modules are counted as not yet reached, not as passing. The open question is whether the generator should copy the field declaration into the test instead, which would trust the extractor's reading of it.

## Decided without the maintainer
The maintainer gave the coordinating agent of this run full autonomy in chat on 2026-10-21 and is away. The coordinator's chat authorization is recorded here; this file does not constitute it, and a statement in a repository file is not maintainer sign-off. Each call below took the most conservative option.

- **The test fails when the field rejects.** The row says each test "expects a rejection". The coordinator's brief for this run says that a generated test that fails shows the library rejects the counterexample, and `intent/2026-10-07-error-witness.md` says a test "is evidence only once it runs and fails as stated". So each test asserts the claim the error disputes, that the target field accepts a value the source admits, and its failure, with the marker text, is the evidence. A failure for any other reason, such as an import error, does not carry the marker, and the CI script counts it as a broken run, not as evidence. The mutation check of the row keeps its meaning: with a value the field accepts, the test passes and the CI script fails.
- **The field comes from the user's code, not from CGV's reading of it.** Rebuilding the field from the extracted constraints would test `BehaviorModel.lean` only as far as the extractor read the declaration right. Importing the model tests the declaration itself, at the cost of the import limit above.
- **The fractional and integral gap is withheld at run time.** Whether the field is integral is read from the real field class, which the extractor does not record for pydantic.
- **The CI change is in this pull request, as a patch for the maintainer to push.** The row names it, and the change needs only a governance note, which the maintainer reviews at merge. GitHub refused the agent's push of the workflow edit, because the agent's token lacks the `workflow` scope, so the pull request body carries the exact diff and the governance note asks the maintainer to push it before merging. This pull request opens and does not merge, so no marker is resolved by an agent.

## Open questions
None. The gaps above stay not yet reached, each with its blocking property and open question.
