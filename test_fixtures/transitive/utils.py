from decimal import Decimal


def compute_offpeak(total: Decimal) -> Decimal:
    """Compute off-peak energy component."""
    ratio = Decimal('0.4')
    result = total * ratio
    return result.quantize(Decimal('0.0001'))  # body analysis -> precision <= 4


def split_energy(offpeak: Decimal) -> Decimal:
    """Split energy applying a minimum precision floor.

    requires: precision(offpeak) <= 10
    ensures: precision(result) <= max(input_precision, 3)
    """
    floor = Decimal('0.001')  # 3dp floor
    if offpeak.as_tuple().exponent > floor.as_tuple().exponent:
        return offpeak.quantize(floor)
    return offpeak
