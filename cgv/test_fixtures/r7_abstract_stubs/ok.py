"""Correct code: every implementation rounds to 2 places; the declarations
have no body to check."""

from abc import ABC, abstractmethod
from decimal import Decimal
from typing import Protocol

from bug import PENNY, Settlement


class Fees(ABC):
    @abstractmethod
    def fee(self, amount: Decimal) -> Decimal:
        """The fee for `amount`."""

    def settle(self, amount: Decimal) -> Settlement:
        return Settlement(fee=self.fee(amount))


class Card(Fees):
    def fee(self, amount: Decimal) -> Decimal:
        return (amount * Decimal("0.02")).quantize(PENNY)


class Pricing:
    def price(self, net: Decimal) -> Decimal:
        pass

    def quote(self, net: Decimal) -> Settlement:
        return Settlement(fee=self.price(net))


class Standard(Pricing):
    def price(self, net: Decimal) -> Decimal:
        return net.quantize(PENNY)


class Rounds(Protocol):
    def price(self, net: Decimal) -> Decimal: ...


class Rounded(Rounds):
    def price(self, net: Decimal) -> Decimal:
        return net.quantize(PENNY)


def quote_rounded(r: Rounds, net: Decimal) -> Settlement:
    return Settlement(fee=r.price(net))
