from decimal import Decimal

from records import Invoice


def make4dp(x: Decimal) -> Decimal:
    return x.quantize(Decimal('0.0001'))


def clamp_floor(p: Decimal, floor: Decimal) -> Decimal:
    # two parameters: the resulting precision cannot be expressed as a
    # dependent bound on a single input, so it is unknown, not checked
    if p.as_tuple().exponent < floor.as_tuple().exponent:
        return p.quantize(floor)
    return p


def make(x: Decimal, floor: Decimal) -> Invoice:
    return Invoice(total=clamp_floor(make4dp(x), floor))
