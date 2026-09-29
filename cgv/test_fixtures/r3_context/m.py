from decimal import Decimal
from pydantic import BaseModel, Field


class Inv(BaseModel):
    total: Decimal = Field(decimal_places=2)
    wide: Decimal = Field(decimal_places=4)


def keep(p: Decimal) -> Decimal:
    """ensures: precision(result) <= max(input_precision, 2)"""
    return p


def four(x: Decimal) -> Decimal:
    return x.quantize(Decimal("0.0001"))


def two(x: Decimal) -> Decimal:
    return x.quantize(Decimal("0.01"))


def make_wide(x: Decimal) -> Inv:
    return Inv(total=Decimal("0"), wide=keep(four(x)))


def make_total(x: Decimal) -> Inv:
    return Inv(total=keep(two(x)), wide=Decimal("0"))
