from decimal import Decimal
from pydantic import BaseModel, Field


class Plan(BaseModel):
    fee: Decimal = Field(decimal_places=2)


def four(x: Decimal) -> Decimal:
    return x.quantize(Decimal("0.0001"))


def build(**kw) -> Plan:
    return Plan(**kw)


def caller(x: Decimal) -> Plan:
    return build(fee=four(x))
