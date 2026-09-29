from decimal import Decimal
from typing import Optional

from django.db import models
from pydantic import BaseModel, Field


class ExportSummary(BaseModel):
    total: Decimal = Field(decimal_places=2)
    largest: Decimal = Field(decimal_places=2)
    line_count: int = Field(ge=1)
    code: str = Field(max_length=10)


class Coupon(models.Model):
    percent_off = models.PositiveSmallIntegerField(null=True)
    code = models.CharField(max_length=10)
