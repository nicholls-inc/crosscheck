from decimal import Decimal

from django.db import models
from pydantic import BaseModel, Field


class Sample(models.Model):
    amount = models.DecimalField(max_digits=6, decimal_places=2)


class Bag(BaseModel):
    ternary: str = Field(max_length=16)
    matched: str = Field(max_length=16)
    attr: str = Field(max_length=16)
    nxt: int
    latest: Decimal = Field(max_digits=6, decimal_places=2)
