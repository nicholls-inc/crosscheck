from decimal import Decimal


def boost(a: Decimal) -> Decimal:
    """ensures: precision(result) <= 4"""
    return a.quantize(Decimal('0.0001'))
