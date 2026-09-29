from decimal import Decimal

from pydantic import BaseModel, Field


class Invoice(BaseModel):
    total: Decimal = Field(max_digits=10, decimal_places=2)
    customer: str = Field(max_length=32)
