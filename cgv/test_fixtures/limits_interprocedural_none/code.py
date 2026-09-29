from decimal import Decimal

from records import Invoice

PRICES = {"a": Decimal('1.00')}


def find(sku):
    return PRICES.get(sku)


def price(sku):
    return find(sku)


def make(sku: str) -> Invoice:
    p = price(sku)
    return Invoice(total=p, tax=Decimal('0'), customer="x")  # bug: p may be None
