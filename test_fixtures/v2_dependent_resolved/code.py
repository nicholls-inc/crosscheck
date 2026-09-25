from decimal import Decimal

from records import Invoice


def make4dp(x: Decimal) -> Decimal:
    return x.quantize(Decimal('0.0001'))


def clamp_floor(p: Decimal) -> Decimal:
    floor = Decimal('0.001')
    if p.as_tuple().exponent < floor.as_tuple().exponent:
        return p.quantize(floor)
    return p


def make(x: Decimal) -> Invoice:
    return Invoice(total=clamp_floor(make4dp(x)))
