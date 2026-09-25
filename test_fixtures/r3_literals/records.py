from decimal import Decimal
from typing import Optional

from pydantic import BaseModel, Field


class Entry(BaseModel):
    amount: Decimal = Field(max_digits=10, decimal_places=2, ge=0)
    limit: Optional[Decimal] = Field(default=None, decimal_places=2)
    label: str = Field(max_length=10)
    note: Optional[str] = Field(default=None, max_length=5)


class Job:
    """Not a data class: a service with class constants."""

    FROZEN = "frozen"
    RATE = Decimal("0.001")
