from decimal import Decimal

from records import Invoice


def make(a: Decimal) -> Invoice:
    return Invoice(total=None, tax=Decimal('0'), customer="x")
