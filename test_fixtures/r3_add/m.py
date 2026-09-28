from decimal import Decimal
from pydantic import BaseModel, Field


class Inv(BaseModel):
    total: Decimal = Field(decimal_places=2)


def money(x: Decimal) -> Decimal:
    return x.quantize(Decimal("0.01"))


def make(a: Decimal, b: Decimal) -> Inv:
    return Inv(total=money(a) + money(b))
