from decimal import Decimal
from typing import Optional

from plain_python.records import InvoiceRecord, LineItem

DISCOUNT_CODES = {"SPRING": 10, "VIP": 25}


def lookup_discount(code: str) -> Optional[int]:
    """Percentage discount for a promotion code, or None if the code is unknown.

    ensures: result <= 100
    """
    return DISCOUNT_CODES.get(code)


def with_tax(amount: Decimal) -> Decimal:
    """Add 20% VAT, keeping four decimal places."""
    return (amount * Decimal('1.2')).quantize(Decimal('0.0001'))


def normalise(amount: Decimal) -> Decimal:
    """Pad to at least two decimal places without discarding precision.

    requires: precision(amount) <= 10
    ensures: precision(result) <= max(input_precision, 2)
    """
    if amount.as_tuple().exponent > -2:
        return amount.quantize(Decimal('0.01'))
    return amount


def customer_label(first: str, last: str) -> str:
    """Display name for invoices.

    ensures: len(result) <= 64
    """
    return f"{last.upper()}, {first}"[:64]


def build_line_item(sku: str, price: Decimal, quantity: int) -> LineItem:
    return LineItem(sku, with_tax(price), quantity)


def record_invoice(first: str, last: str, subtotal: Decimal, code: str) -> InvoiceRecord:
    total = normalise(with_tax(subtotal))
    return InvoiceRecord(
        customer=customer_label(first, last),
        total=total,
        discount_pct=lookup_discount(code),
    )
