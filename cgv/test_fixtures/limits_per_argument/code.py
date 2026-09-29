from decimal import Decimal

from records import Invoice


def with_tax(a: Decimal) -> Decimal:
    return (a * Decimal('1.2')).quantize(Decimal('0.0001'))

def combine(amount: Decimal, rate: Decimal) -> Decimal:
    """requires: precision(amount) <= 2"""
    return amount

def make(a: Decimal, b: Decimal) -> Invoice:
    return Invoice(total=combine(a.quantize(Decimal('0.01')), with_tax(b)), tax=Decimal('0'), customer="x")


def make_bad(a: Decimal, b: Decimal) -> Invoice:
    # with_tax's 4dp result binds `amount`, which requires <= 2: bug at the combine hop
    return Invoice(total=combine(with_tax(b), a.quantize(Decimal('0.01'))), tax=Decimal('0'), customer="x")
