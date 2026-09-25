from decimal import Decimal
from pydantic import BaseModel


class Band(BaseModel):
    rate: Decimal


def rate_for(kind: str) -> Decimal:
    for k in ("peak",):
        if k == kind:
            return Decimal("0.35")
    raise ValueError(kind)


def rate_while(kind: str) -> Decimal:
    while True:
        return Decimal("0.35")


def rate_try(kind: str) -> Decimal:
    try:
        return Decimal(kind)
    except Exception:
        raise


def make(kind: str) -> Band:
    return Band(rate=rate_for(kind))


def make2(kind: str) -> Band:
    return Band(rate=rate_while(kind))


def make3(kind: str) -> Band:
    return Band(rate=rate_try(kind))
