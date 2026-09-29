from decimal import Decimal

from records import Reading


def correct(pk: int, v: Decimal) -> None:
    r = Reading.objects.get(pk=pk)
    r.kwh = v.quantize(Decimal('0.000001'))  # bug: 6dp > 3dp
