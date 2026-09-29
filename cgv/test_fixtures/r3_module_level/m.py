from decimal import Decimal
from pydantic import BaseModel, Field


class Plan(BaseModel):
    fee: Decimal = Field(decimal_places=2)


def four(x: Decimal) -> Decimal:
    return x.quantize(Decimal("0.0001"))


DEFAULT = Plan(fee=four(Decimal("1")))
