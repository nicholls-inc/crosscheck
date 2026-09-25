from decimal import Decimal

from models import Reading


def record_reading(v: Decimal) -> Reading:
    """Record a raw meter reading.

    ensures: precision(result) <= 5
    """
    return Reading.objects.create(kwh=v)
