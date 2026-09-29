from decimal import Decimal
from pydantic import BaseModel


class Band(BaseModel):
    rate: Decimal


def rate_for(kind: str) -> Decimal:
    if kind == "peak":
        return Decimal("0.35")
    else:
        raise ValueError(kind)


def make(kind: str) -> Band:
    return Band(rate=rate_for(kind))
