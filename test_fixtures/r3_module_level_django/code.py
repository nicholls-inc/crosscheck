from decimal import Decimal

from models import Price


def four(x: Decimal) -> Decimal:
    return x.quantize(Decimal("0.0001"))


DEFAULT_PRICE = Price(value=four(Decimal("1")))
