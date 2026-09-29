from decimal import Decimal
from pydantic import BaseModel, Field


class S(BaseModel):
    e: Decimal = Field(decimal_places=3)


def five() -> Decimal:
    """ensures: precision(result) <= 5"""
    return Decimal("1.00000")


def keep(e: Decimal) -> Decimal:
    """ensures: precision(result) <= max(input_precision, 2)"""
    return e


def settle() -> S:
    return S(e=keep(five()))
