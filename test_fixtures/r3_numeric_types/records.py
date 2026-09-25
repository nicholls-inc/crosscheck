from dataclasses import dataclass
from decimal import Decimal

from django.db import models
from pydantic import BaseModel, ConfigDict, Field, StrictInt


class Lax(BaseModel):
    price: Decimal = Field(decimal_places=2)
    count: int
    ratio: float


class StrictModel(BaseModel):
    model_config = ConfigDict(strict=True)

    price: Decimal


class StrictField(BaseModel):
    count: int = Field(strict=True)
    other: StrictInt


class Reading(models.Model):
    kwh = models.FloatField()
    cost = models.DecimalField(max_digits=10, decimal_places=2)
    count = models.IntegerField()


@dataclass
class Row:
    price: Decimal
