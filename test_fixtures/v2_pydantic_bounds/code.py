from records import Item


def make_qty(n: int) -> int:
    """ensures: result >= 0"""
    return n


def calc_pct(n: int) -> int:
    """ensures: result >= 0
    ensures: result <= 100
    """
    return n


def make_item(n: int, m: int) -> Item:
    return Item(qty=make_qty(n), pct=calc_pct(m))
