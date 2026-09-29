from decimal import Decimal

from records import Invoice


def make(a: Decimal, b: Decimal) -> Invoice:
    return Invoice(total=a.quantize(Decimal('0.01')), tax=b.quantize(Decimal('0.0001')), customer="x")
