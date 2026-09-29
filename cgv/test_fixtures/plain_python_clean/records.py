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
    customer: Annotated[str, Field(max_length=64)]
    total: Decimal = Field(max_digits=12, decimal_places=4)
    discount_pct: Optional[int] = Field(default=None, le=100)
