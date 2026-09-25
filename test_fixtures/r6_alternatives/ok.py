"""Correct code: no errors (alternatives that are all within the bounds)."""

from decimal import Decimal

from models import StockItem


def apply_ok(item: StockItem, c: bool) -> None:
    item.qty = 5 if c else 0
    item.save()


def local_defs_ok(raw: str, c: bool) -> StockItem:
    code = raw[:6]
    if c:
        code = "SHORT"
    return StockItem.objects.create(qty=1, code=code, fee=Decimal("0.00"))


def fee_ok(amount: Decimal, c: bool) -> StockItem:
    fee = amount.quantize(Decimal("0.01")) if c else Decimal("1.50")
    return StockItem.objects.create(qty=1, code="A", fee=fee)
