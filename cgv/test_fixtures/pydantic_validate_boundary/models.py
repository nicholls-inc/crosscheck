from decimal import Decimal
from typing import Annotated, Optional

from pydantic import BaseModel, Field, PlainValidator, SkipValidation, WrapValidator


class Reading(BaseModel):
    enabled: bool
    label: str
    total: Decimal = Field(decimal_places=2)


class Skipped(BaseModel):
    label: SkipValidation[str]


class Plain(BaseModel):
    label: Annotated[str, PlainValidator(lambda v: v)]


class Wrapped(BaseModel):
    label: Annotated[str, WrapValidator(lambda v, handler: v)]


class Legacy(BaseModel):
    label: str = None


def maybe_label(d: dict) -> Optional[str]:
    return d.get("label")


def maybe_note(d: dict) -> Optional[str]:
    return d.get("note")


def four(x: Decimal) -> Decimal:
    return x.quantize(Decimal("0.0001"))


def four_more(x: Decimal) -> Decimal:
    return x.quantize(Decimal("0.0001"))


def construct_flag() -> Reading:
    # BUG: a typed constructor call, and a type checker rejects a str for a bool
    return Reading(enabled="true", label="a", total=Decimal("1.00"))


def construct_label(d: dict) -> Reading:
    # BUG: a typed constructor call, and the label may be None
    return Reading(enabled=True, label=maybe_label(d), total=Decimal("1.00"))


def construct_total(x: Decimal) -> Reading:
    # BUG: a typed constructor call with 4 decimal places
    return Reading(enabled=True, label="a", total=four(x))


def copy_label(r: Reading, d: dict) -> Reading:
    # BUG: model_copy does not validate, so the None is stored
    return r.model_copy(update={"label": maybe_note(d)})


def validate_total(x: Decimal) -> Reading:
    # BUG: decimal_places counts digits after trailing zeros are dropped, so 1.2300 is stored
    return Reading.model_validate({"enabled": True, "label": "a", "total": four_more(x)})


def validate_skipped(d: dict) -> Skipped:
    # BUG: SkipValidation stores the None
    return Skipped.model_validate({"label": maybe_label(d)})


def validate_plain(d: dict) -> Plain:
    # BUG: a PlainValidator replaces validation, and this one stores the None
    return Plain.model_validate({"label": maybe_label(d)})


def validate_wrapped(d: dict) -> Wrapped:
    # BUG: a WrapValidator may skip validation, and this one stores the None
    return Wrapped.model_validate({"label": maybe_label(d)})


def validate_legacy(d: dict) -> Legacy:
    # BUG: pydantic v1 reads `str = None` as Optional, so parse_obj stores the None
    return Legacy.parse_obj({"label": maybe_label(d)})
