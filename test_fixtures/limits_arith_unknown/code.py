from decimal import Decimal

from records import Invoice


def make(a: Decimal, rate: Decimal) -> Invoice:
    x = a.quantize(Decimal('0.01')) * rate
    return Invoice(total=x, tax=Decimal('0'), customer="x")
