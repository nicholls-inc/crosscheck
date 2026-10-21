from dataclasses import dataclass
from decimal import Decimal
from typing import Literal

from pydantic import BaseModel, Field


class Order(BaseModel):
    ref: str = Field(max_length=4)
    amount: Decimal = Field(decimal_places=2)
    qty: int = Field(le=10)
    discount: float = Field(ge=0)
    channel: Literal["web", "shop"]
    label: str
    count: int = Field(le=5)


@dataclass
class Draft:
    title: str
