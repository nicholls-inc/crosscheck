from decimal import Decimal
from typing import Optional
from pydantic import BaseModel, Field


class P(BaseModel):
    amount: Decimal = Field(decimal_places=2)
    limit: Optional[Decimal] = Field(default=None, decimal_places=2)


def round2(x: Decimal) -> Decimal:
    return x.quantize(Decimal("0.01"))


def make(amount: Decimal) -> P:
    return P(amount=round2(amount), limit=None)
