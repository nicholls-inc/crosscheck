from decimal import Decimal

from metering.models import EnergyRecord


def compute_offpeak(total: Decimal) -> EnergyRecord:
    """Compute the off-peak energy component.

    ensures: precision(result) <= 3
    """
    offpeak = (total * Decimal("0.4")).quantize(Decimal("0.001"))
    return split_energy(offpeak)


def split_energy(offpeak: Decimal) -> EnergyRecord:
    """Split energy applying a minimum precision floor.

    requires: precision(offpeak) <= 10
    ensures: precision(result) <= max(input_precision, 3)
    """
    floor = Decimal("0.001")
    if offpeak.as_tuple().exponent > floor.as_tuple().exponent:
        energy_val = offpeak.quantize(floor)
    else:
        energy_val = offpeak
    return EnergyRecord.objects.create(energy=energy_val)
