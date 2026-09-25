from decimal import Decimal

from records import Order


def combine(base: Decimal, surcharge: Decimal) -> Decimal:
    """requires: precision(base) <= 2
    requires: precision(surcharge) <= 4
    """
    return base + surcharge


def make4dp(x: Decimal) -> Decimal:
    return x.quantize(Decimal('0.0001'))


def make(a: Decimal, b: Decimal) -> Order:
    # base=make4dp(a) violates combine's `base` precondition (4dp > 2dp);
    # surcharge=b.quantize(...) satisfies combine's `surcharge` precondition.
    return Order(total=combine(make4dp(a), b.quantize(Decimal('0.0001'))))
