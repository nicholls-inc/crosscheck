from decimal import Decimal

from m import R


def on_the_bounds() -> R:
    # SAFE: every value is exactly on its (inclusive) bound
    return R(ratio=0.5, amt=Decimal("10.0"), amt2=Decimal("0"))


def inside() -> R:
    # SAFE
    return R(ratio=0.25, amt=Decimal("-3.5"), amt2=Decimal("0.001"))
