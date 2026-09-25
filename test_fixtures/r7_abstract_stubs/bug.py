"""Round 7: `@abstractmethod` methods, `Protocol` methods and stub bodies
(`...`, `pass` or a docstring under a non-None return annotation, `raise
NotImplementedError`) are not dispatch targets when a project subclass
implements the method. Final verification projects/g12.
"""

from abc import ABC, abstractmethod
from decimal import Decimal

from pydantic import BaseModel, Field

PENNY = Decimal("0.01")


class Settlement(BaseModel):
    fee: Decimal = Field(decimal_places=2)


class Gateway(ABC):
    @abstractmethod
    def fee(self, amount: Decimal) -> Decimal: ...

    def settle(self, amount: Decimal) -> Settlement:
        return Settlement(fee=self.fee(amount))


class Stripe(Gateway):
    def fee(self, amount: Decimal) -> Decimal:
        return (amount * Decimal("0.014")).quantize(PENNY)


class Legacy(Gateway):
    def fee(self, amount: Decimal) -> Decimal:
        # BUG: 3 places
        return (amount * Decimal("0.025")).quantize(Decimal("0.001"))


class Tariff:
    def price(self, net: Decimal) -> Decimal:
        raise NotImplementedError


class Flat(Tariff):
    def price(self, net: Decimal) -> Decimal:
        # BUG: 4 places
        return net.quantize(Decimal("0.0001"))


def charge(tariff: Tariff, net: Decimal) -> Settlement:
    return Settlement(fee=tariff.price(net))
