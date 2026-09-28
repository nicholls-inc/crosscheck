from decimal import Decimal
from pydantic import BaseModel, Field

from pkg.consts import CENT
from pkg import consts


class P(BaseModel):
    x: Decimal = Field(decimal_places=2)


def r(v: Decimal) -> Decimal:
    return v.quantize(CENT)


def r2(v: Decimal) -> Decimal:
    return v.quantize(consts.CENT)


def a(v: Decimal) -> P:
    return P(x=r(v))


def b(v: Decimal) -> P:
    return P(x=r2(v))
