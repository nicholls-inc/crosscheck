from decimal import Decimal

from records import Invoice


PRICES = {"a": Decimal('1.00')}

def price_for(sku):
    return PRICES.get(sku)

def make(sku: str) -> Invoice:
    return Invoice(total=price_for(sku), tax=Decimal('0'), customer="x")
