from decimal import Decimal
from typing import Optional

from django.db import models
from pydantic import BaseModel, Field


class Invoice(BaseModel):
    total: Decimal = Field(decimal_places=2)
    number: str = Field(max_length=8)

    @classmethod
    def from_raw(cls, raw: Decimal) -> "Invoice":
        # BUG: 4 decimal places via cls(...) (adversarial2 min/m19)
        return cls(total=raw.quantize(Decimal("0.0001")), number="INV-1")

    @classmethod
    def blank(cls) -> "Invoice":
        # BUG: an 11-character number
        return cls(total=Decimal("0.00"), number="INV-0000001")

    @classmethod
    def rounded(cls, raw: Decimal) -> "Invoice":
        # SAFE
        return cls(total=raw.quantize(Decimal("0.01")), number="INV-2")


class Refund(models.Model):
    reason = models.CharField(max_length=20)

    @classmethod
    def none_reason(cls) -> "Refund":
        # BUG: None into a non-null column via cls.objects.create
        return cls.objects.create(reason=None)

    @classmethod
    def make(cls, reason: str) -> "Refund":
        # SAFE: a str parameter (length unknown: a warning)
        return cls.objects.create(reason=reason)


def direct(raw: Decimal) -> Invoice:
    # BUG: the same as from_raw, called directly
    return Invoice(total=raw.quantize(Decimal("0.0001")), number="INV-3")


def maybe_reason(r: Optional[str]) -> Refund:
    return Refund.make(r)
