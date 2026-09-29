from decimal import Decimal

from pydantic import BaseModel, Field


class Invoice(BaseModel):
    total: Decimal = Field(max_digits=12, decimal_places=6)
