from dataclasses import dataclass
from decimal import Decimal
from typing import Annotated, Optional

from pydantic import BaseModel, Field


@dataclass
class LineItem:
    sku: str
    unit_price: Decimal
    quantity: int


class InvoiceRecord(BaseModel):
    customer: Annotated[str, Field(max_length=32)]
    total: Decimal = Field(max_digits=10, decimal_places=2)
    discount_pct: int = Field(le=100)
    note: Optional[str] = None
