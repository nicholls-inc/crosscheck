import re
from decimal import Decimal
from typing import List

from pydantic import BaseModel, Field

CODE = re.compile(r"ACC-(\w+)")


class Customer(BaseModel):
    account_code: str = Field(max_length=6)


class Totals(BaseModel):
    subtotal: Decimal = Field(decimal_places=2)


class Parser:
    PATTERN = re.compile(r"(\d+)")

    def number(self, text: str) -> Customer:
        # BUG: a compiled class-level pattern's match may be None
        return Customer(account_code=self.PATTERN.match(text))


def code_bad(text: str):
    # BUG: `m and ...` is None when there is no match (adversarial2 n14)
    return (m := CODE.match(text)) and m.group(1)[:6]


def code_local(text: str):
    # BUG: the same with a local pattern
    pat = re.compile(r"X-(\w+)")
    return pat.search(text)


def line_total(x: Decimal) -> Decimal:
    return x.quantize(Decimal("0.0001"))


def two(x: Decimal) -> Decimal:
    return x.quantize(Decimal("0.01"))


def build_bad(text: str) -> Customer:
    return Customer(account_code=code_bad(text))


def build_local(text: str) -> Customer:
    return Customer(account_code=code_local(text))


def totals(xs: List[Decimal]) -> Totals:
    # BUG: summing 4dp line totals gives a 4dp subtotal (adversarial2 n04)
    return Totals(subtotal=sum((line_total(x) for x in xs), Decimal("0")))


def totals_list(a: Decimal, b: Decimal) -> Totals:
    # BUG: the same over a list display
    return Totals(subtotal=sum([line_total(a), two(b)]))
