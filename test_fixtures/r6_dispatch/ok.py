"""Correct code: every override rounds to 2 places."""

from decimal import Decimal

from bug import Quote


class Retail:
    def round(self, v: Decimal) -> Decimal:
        return v.quantize(Decimal("0.01"))

    def price(self, net: Decimal) -> Decimal:
        return self.round(net)


class Wholesale(Retail):
    def round(self, v: Decimal) -> Decimal:
        return (v * Decimal("0.9")).quantize(Decimal("0.01"))


class Promo(Retail):
    def price(self, net: Decimal) -> Decimal:
        return super().price(net) - Decimal("0.50")


def quote_ok(engine: Retail, net: Decimal) -> Quote:
    return Quote(total=engine.price(net))


def quote_promo(net: Decimal) -> Quote:
    return Quote(total=Promo().price(net))
