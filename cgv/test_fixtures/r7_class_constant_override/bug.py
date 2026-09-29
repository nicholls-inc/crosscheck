"""Round 7: a class constant read as `self.X` / `cls.X` in an inherited
method takes every value it has in the class and in project subclasses
overriding it (class hierarchy, as for method dispatch). Final verification
min/cc1, min/cc2, projects/g12.
"""

from decimal import Decimal

from pydantic import BaseModel, Field


class Settlement(BaseModel):
    code: str = Field(max_length=4)
    amount: Decimal = Field(decimal_places=2)


class Bank:
    code = "base"

    def settle(self, x: Decimal) -> Settlement:
        # BUG: a Legacy instance settles with code "legacy-bank" (11 > 4)
        return Settlement(code=self.code, amount=x.quantize(Decimal("0.01")))


class Legacy(Bank):
    code = "legacy-bank"


def run(x: Decimal) -> Settlement:
    return Legacy().settle(x)


class Rounder:
    Q = Decimal("0.01")

    def settle(self, x: Decimal) -> Settlement:
        # BUG: Precise.Q has 4 places
        return Settlement(code="rnd", amount=x.quantize(self.Q))

    @classmethod
    def settle_cls(cls, x: Decimal) -> Settlement:
        # BUG: the same through `cls`
        return Settlement(code="rnd", amount=x.quantize(cls.Q))


class Precise(Rounder):
    Q = Decimal("0.0001")


class Grand(Precise):
    pass


class Computed:
    code = "cmp"

    def settle(self, x: Decimal) -> Settlement:
        # Unknown (warning): a subclass binds `code` to a computed value.
        return Settlement(code=self.code, amount=x.quantize(Decimal("0.01")))


def make_code() -> str:
    return "computed-code"


class ComputedSub(Computed):
    code = make_code()
