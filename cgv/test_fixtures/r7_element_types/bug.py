"""Round 7: elements of parameters annotated `list[X]`, `Sequence[X]`,
`Iterable[X]`, `tuple[X, ...]`, `set[X]` are instances of X in `for` loops
and comprehensions, so a field read through them has the field's declared
contracts. Final verification min/s1, projects/g02.
"""

from decimal import Decimal
from typing import Iterable, Sequence

from pydantic import BaseModel, Field

RATE = Decimal("0.20")


class Line(BaseModel):
    net: Decimal = Field(decimal_places=2)


class Invoice(BaseModel):
    total: Decimal = Field(decimal_places=2)


def vat_total(lines: list[Line]) -> Invoice:
    # BUG: 2 places times 2 places
    return Invoice(total=sum(ln.net * RATE for ln in lines))


def vat_total_start(lines: Sequence[Line]) -> Invoice:
    # BUG: the same with a start value
    return Invoice(total=sum((ln.net * RATE for ln in lines), Decimal("0.00")))


def largest_vat(lines: tuple[Line, ...]) -> Invoice:
    # BUG: max over a list comprehension
    return Invoice(total=max([ln.net * RATE for ln in lines]))


def last_vat(lines: Iterable[Line]) -> Invoice:
    for ln in lines:
        # BUG: a loop variable
        return Invoice(total=ln.net * RATE)
    return Invoice(total=Decimal("0.00"))
