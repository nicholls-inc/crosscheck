from decimal import Decimal
from pydantic import BaseModel, Field


class PriceIn(BaseModel):
    price: Decimal = Field(decimal_places=2)


def float_price(p: float) -> float:
    return round(p, 2)


def to_schema(p: float) -> PriceIn:
    return PriceIn(price=float_price(p))
