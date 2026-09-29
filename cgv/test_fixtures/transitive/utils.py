from decimal import Decimal

from transitive.models import EnergyRecord


def compute_offpeak(total: Decimal) -> EnergyRecord:
    """Compute off-peak energy component.

    ensures: precision(result) <= 4
    """
    ratio = Decimal('0.4')
    offpeak = (total * ratio).quantize(Decimal('0.0001'))
    return split_energy(offpeak)


def split_energy(offpeak: Decimal) -> EnergyRecord:
    """Split energy applying a minimum precision floor.

    requires: precision(offpeak) <= 10
    ensures: precision(result) <= max(input_precision, 3)
    """
    floor = Decimal('0.001')  # 3dp floor
    if offpeak.as_tuple().exponent > floor.as_tuple().exponent:
        energy_val = offpeak.quantize(floor)
    else:
        energy_val = offpeak
    return EnergyRecord.objects.create(energy=energy_val)
