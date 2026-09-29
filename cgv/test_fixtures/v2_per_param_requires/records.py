from decimal import Decimal

from pydantic import BaseModel, Field


class Order(BaseModel):
    total: Decimal = Field(max_digits=10, decimal_places=2)
