from decimal import Decimal

from records import Invoice


def make(a: Decimal) -> Invoice:
    data = {"total": a.quantize(Decimal('0.0001')), "tax": Decimal('0'), "customer": "x"}
    return Invoice(**data)
