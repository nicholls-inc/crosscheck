"""Correct code: sums and maxima of 2-place fields have 2 places."""

from decimal import Decimal
from typing import Iterable, Sequence

from bug import Invoice, Line


def net_total(lines: list[Line]) -> Invoice:
    return Invoice(total=sum(ln.net for ln in lines))


def net_total_start(lines: Sequence[Line]) -> Invoice:
    return Invoice(total=sum((ln.net for ln in lines), Decimal("0.00")))


def largest(lines: tuple[Line, ...]) -> Invoice:
    return Invoice(total=max(ln.net for ln in lines))


def distinct(lines: set[Line]) -> Invoice:
    return Invoice(total=sum({ln.net for ln in lines}))


def first(lines: Iterable[Line]) -> Invoice:
    for ln in lines:
        return Invoice(total=ln.net)
    return Invoice(total=Decimal("0.00"))


def doubled(lines: list[Line]) -> Invoice:
    return Invoice(total=sum(ln.net * 2 for ln in lines))
