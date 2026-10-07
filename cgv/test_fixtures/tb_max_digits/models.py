from decimal import Decimal

from django.db import models
from pydantic import BaseModel, Field


class Price(models.Model):
    amount = models.DecimalField(max_digits=5, decimal_places=2)


class Quote(BaseModel):
    amount: Decimal = Field(max_digits=5, decimal_places=2)
