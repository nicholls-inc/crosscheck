from decimal import Decimal
from pydantic import BaseModel, Field


class Refund(BaseModel):
    amount: Decimal = Field(decimal_places=2)


def third(a: Decimal) -> Decimal:
    return (a / 3).quantize(Decimal("0.001"))


def refund_a(a: Decimal) -> Refund:
    return Refund(amount=third(a))


def refund_b(a: Decimal) -> Refund:
    return Refund(**{"amount": third(a)})


def refund_c(a: Decimal) -> Refund:
    r = Refund(amount=Decimal("0"))
    r.amount = third(a)
    return r
