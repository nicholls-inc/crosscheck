from decimal import Decimal

from nullable.models import Invoice


def apply_discount(subtotal: Decimal, discount_pct: Decimal) -> Invoice:
    """Apply discount if applicable.

    ensures: precision(result) <= 4
    """
    if discount_pct <= 0:
        return None  # bug: returns None under a non-Optional `-> Invoice`
    discounted = (subtotal * discount_pct).quantize(Decimal('0.0001'))  # 4dp > 2dp
    return Invoice.objects.create(
        total=discounted,
        discount=discounted,
    )
