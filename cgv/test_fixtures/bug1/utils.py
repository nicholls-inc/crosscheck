from decimal import Decimal

from bug1.models import EnergyRecord


def split_energy(total: Decimal, off_peak: Decimal) -> EnergyRecord:
    """Split energy across a month boundary."""
    ratio = Decimal('0.6')
    period1 = total * ratio
    period2 = total - period1
    return EnergyRecord.objects.create(
        energy=period1.quantize(Decimal('0.000001')),  # quantize to 6dp
        off_peak_energy=period2.quantize(Decimal('0.000001')),  # quantize to 6dp
    )
