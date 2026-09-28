"""Correct code: every override fits the field."""

from decimal import Decimal

from bug import Settlement


class Gateway:
    code = "gw"
    Q = Decimal("0.01")

    def settle(self, x: Decimal) -> Settlement:
        return Settlement(code=self.code, amount=x.quantize(self.Q))


class Stripe(Gateway):
    code = "strp"


class Adyen(Gateway):
    code = "adyn"
    Q = Decimal("0.1")


class Local(Adyen):
    pass
