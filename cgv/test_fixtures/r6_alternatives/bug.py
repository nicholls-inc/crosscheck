"""Round 6: a value with a few alternatives (the branches of `a if c else b`,
the reaching definitions of a local) is one edge per alternative at the
same site, so a branch that certainly violates is an error even when
another branch is unknown (adversarial2 n12, final f15).
"""

from decimal import Decimal

from models import StockItem


def apply(item: StockItem, x: int, c: bool) -> None:
    # BUG: -1 when not c (the other branch's lower bound is unknown)
    item.qty = x - 1 if c else -1
    item.save()


def local_defs(raw: str, c: bool) -> StockItem:
    # BUG: one reaching definition is 9 characters
    code = raw[:6]
    if c:
        code = "TOOLONG99"
    return StockItem.objects.create(qty=1, code=code, fee=Decimal("0.00"))


def fee_of(amount: Decimal, c: bool) -> StockItem:
    # BUG: the quantized branch is 3 places; the other is unknown
    fee = amount.quantize(Decimal("0.001")) if c else amount * 2
    return StockItem.objects.create(qty=1, code="A", fee=fee)
