from decimal import Decimal

from shop.records import Invoice


def make(a: Decimal) -> Invoice:
    return Invoice(total=a.quantize(Decimal('0.0001')))  # 4dp <= 6dp: fine
