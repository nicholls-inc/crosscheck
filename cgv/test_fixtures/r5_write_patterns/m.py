import dataclasses
from dataclasses import dataclass, replace
from decimal import Decimal
from typing import Optional

from django.db import models
from pydantic import BaseModel, Field


def maybe(d: dict) -> Optional[str]:
    return d.get("x")


def four(x: Decimal) -> Decimal:
    return x.quantize(Decimal("0.0001"))


@dataclass
class Row:
    name: str


class Inv(BaseModel):
    total: Decimal = Field(decimal_places=2)


class Payout(models.Model):
    reference = models.CharField(max_length=12)
    fee = models.DecimalField(max_digits=10, decimal_places=2)


class PayoutSerializer:
    def create(self, validated_data: dict) -> Payout:
        return Payout.objects.create(**validated_data)


def a(r: Row, d: dict) -> Row:
    # BUG: dataclasses.replace writes a possibly-None name (adversarial2 min/u1)
    return replace(r, name=maybe(d))


def b(r: Row, d: dict) -> Row:
    # BUG: the same, qualified
    return dataclasses.replace(r, name=maybe(d))


def c(i: Inv, x: Decimal) -> Inv:
    # BUG: model_copy(update=...) with 4 decimal places
    return i.model_copy(update={"total": four(x)})


def e(x: Decimal) -> Inv:
    # BUG: model_validate of a dict display
    return Inv.model_validate({"total": four(x)})


def f(x: Decimal) -> Inv:
    # BUG: the explicit key after a spread wins
    base = {"total": Decimal("1.00")}
    return Inv(**{**base, "total": four(x)})


def g(r: Row, d: dict) -> None:
    # BUG: setattr with a literal name
    setattr(r, "name", maybe(d))


def make_payout(x: Decimal) -> Payout:
    # BUG: a dict passed to a method that splats it into create(**data) (adversarial2 n06)
    ser = PayoutSerializer()
    return ser.create({"reference": "PO-1", "fee": four(x)})
