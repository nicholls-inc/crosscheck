from decimal import Decimal

from records import Invoice


def update(inv: Invoice, a: Decimal) -> None:
    inv.total = a.quantize(Decimal('0.0001'))
