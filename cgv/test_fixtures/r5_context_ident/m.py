from decimal import Decimal
from pydantic import BaseModel, Field


class Inv(BaseModel):
    total: Decimal = Field(decimal_places=2)


def four(x: Decimal) -> Decimal:
    return x.quantize(Decimal("0.0001"))


def two(x: Decimal) -> Decimal:
    return x.quantize(Decimal("0.01"))


def ident(p: Decimal) -> Decimal:
    return p


def caller_ok(x: Decimal) -> Inv:
    return Inv(total=ident(two(x)))


def caller_bad(x: Decimal) -> Inv:
    return Inv(total=ident(four(x)))
