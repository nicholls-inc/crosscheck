import enum
from dataclasses import dataclass
from decimal import Decimal
from typing import List, Optional

from django.db import models
from django.db.models import F


class Colour(enum.Enum):
    RED = "red"
    BLUE = "blue"


class Wallet(models.Model):
    balance = models.DecimalField(max_digits=10, decimal_places=2)
    points = models.IntegerField()
    label = models.CharField(max_length=40)
    note = models.CharField(max_length=40, null=True)


@dataclass
class Reading:
    meter: str
    kwh: Decimal
    extra: Optional[str]


@dataclass
class Summary:
    """Every field is non-null: the writes below must not warn about nullability."""

    total: object
    count: object
    biggest: object
    items: object
    text: object
    colour: object
    meter: object


def summarise(xs: List[Decimal], r: Reading, w: Wallet, name, n: int) -> Summary:
    total = Decimal("0")
    for x in xs:
        total += x
    return Summary(
        total=total,
        count=len(xs) + n,
        biggest=max(xs),
        items=sorted(xs),
        text=name.strip() + "%s" % n,
        colour=Colour.RED,
        meter=r.meter,
    )


def credit(w: Wallet, amount: Decimal) -> None:
    # F() expressions and arithmetic: never None.
    Wallet.objects.filter(pk=w.pk).update(points=F("points") + 1, label=w.label)


def optional_read(r: Reading) -> Wallet:
    # BUG: an Optional field read into a non-null column
    return Wallet.objects.create(balance=Decimal("0.00"), points=0, label=r.extra)


def min_or_none(xs: List[int]) -> Wallet:
    # BUG: `default=None` makes min() nullable
    return Wallet.objects.create(balance=Decimal("0.00"), points=min(xs, default=None), label="x")
