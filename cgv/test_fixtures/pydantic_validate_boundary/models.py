from decimal import Decimal
from typing import Optional

from pydantic import BaseModel, Field


class Reading(BaseModel):
    enabled: bool
    label: str
    total: Decimal = Field(decimal_places=2)


def maybe_label(d: dict) -> Optional[str]:
    return d.get("label")


def four(x: Decimal) -> Decimal:
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
    return r.model_copy(update={"label": maybe_label(d)})
