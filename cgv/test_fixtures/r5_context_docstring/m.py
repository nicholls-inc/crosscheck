from decimal import Decimal
from pydantic import BaseModel, Field


class Inv(BaseModel):
    total: Decimal = Field(decimal_places=2)


def four(x: Decimal) -> Decimal:
    return x.quantize(Decimal("0.0001"))


def two(x: Decimal) -> Decimal:
    return x.quantize(Decimal("0.01"))


def keep(p: Decimal) -> Decimal:
    """ensures: precision(result) <= max(input_precision, 2)"""
    return p


def store(v: Decimal) -> Inv:
    return Inv(total=keep(v))


def caller_ok(x: Decimal) -> Inv:
    return store(two(x))


def caller_bad(x: Decimal) -> Inv:
    return store(four(x))
