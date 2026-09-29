from decimal import Decimal

from django.db import models
from pydantic import BaseModel, Field


class Reading(models.Model):
    kwh = models.DecimalField(max_digits=8, decimal_places=3)


class Meter(BaseModel):
    total: Decimal = Field(max_digits=10, decimal_places=2)

    def bump(self, delta: Decimal) -> None:
        self.total = delta.quantize(Decimal('0.0001'))  # bug: 4dp > 2dp
