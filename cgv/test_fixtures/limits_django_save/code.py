from decimal import Decimal

from models import Reading


def correct(r: Reading, v: Decimal) -> None:
    r.kwh = v.quantize(Decimal('0.000001'))
    r.save()
