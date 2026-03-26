from decimal import Decimal


def split_energy(total: Decimal, off_peak: Decimal) -> tuple[Decimal, Decimal]:
    """Split energy across a month boundary."""
    ratio = Decimal('0.6')
    period1 = total * ratio
    period2 = total - period1
    return (
        period1.quantize(Decimal('0.000001')),  # quantize to 6dp
        period2.quantize(Decimal('0.000001')),  # quantize to 6dp
    )
