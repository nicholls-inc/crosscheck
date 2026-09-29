"""Round 6: a method call dispatches to every override in project subclasses
of the receiver's class (class hierarchy analysis), including `self.m()` in
an inherited method; `super().m()` is the parent's method; `Cls().m()`
resolves on `Cls`. Final pass min/f13a, min/f13b, projects/f13.
"""

from decimal import Decimal

from pydantic import BaseModel, Field


class Quote(BaseModel):
    total: Decimal = Field(decimal_places=2)


class Engine:
    def round(self, v: Decimal) -> Decimal:
        return v.quantize(Decimal("0.01"))

    def price(self, net: Decimal) -> Decimal:
        return self.round(net)


class Precise(Engine):
    def round(self, v: Decimal) -> Decimal:
        return v.quantize(Decimal("0.0001"))


def quote(net: Decimal) -> Quote:
    # BUG: Precise().price runs Engine.price, whose self.round is Precise.round
    return Quote(total=Precise().price(net))


class Tariff:
    def price(self, net: Decimal) -> Decimal:
        return net.quantize(Decimal("0.01"))


class PreciseTariff(Tariff):
    def price(self, net: Decimal) -> Decimal:
        return net.quantize(Decimal("0.0001"))


def quote_with(tariff: Tariff, net: Decimal) -> Quote:
    # BUG: tariff may be a PreciseTariff
    return Quote(total=tariff.price(net))


class Discounted(Tariff):
    def price(self, net: Decimal) -> Decimal:
        # BUG: 2 places minus a 3-place literal
        return super().price(net) - Decimal("0.005")


def quote_discounted(net: Decimal) -> Quote:
    return Quote(total=Discounted().price(net))
