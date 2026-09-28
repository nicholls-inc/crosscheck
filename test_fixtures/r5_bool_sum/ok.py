from decimal import Decimal
from typing import List

from m import CODE, Customer, Totals, two


def code(text: str) -> str:
    if (m := CODE.match(text)) is None:
        raise ValueError(text)
    return m.group(1)[:6]


def code_or(text: str):
    # SAFE: `or` with a non-None literal
    return CODE.match(text) and "ACC" or "NONE"


def build(text: str) -> Customer:
    return Customer(account_code=code(text))


def build_or(text: str) -> Customer:
    return Customer(account_code=code_or(text))


def totals_ok(xs: List[Decimal]) -> Totals:
    return Totals(subtotal=sum((two(x) for x in xs), Decimal("0.00")))


def totals_ints(n: int) -> Totals:
    return Totals(subtotal=sum([1, 2, n]))
