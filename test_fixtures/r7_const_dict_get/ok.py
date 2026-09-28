"""Correct code: every value and the default fit."""

from decimal import Decimal

import bug
from bug import Quote

SHORT = {"EUR": "Euro", "USD": "Dollar"}
FEES = {"EUR": Decimal("0.25"), "USD": Decimal("0.3")}


def quote_short(ccy: str) -> Quote:
    return Quote(rate=FEES.get(ccy, Decimal("0.00")), label=SHORT.get(ccy, "other"))


def quote_module(ccy: str) -> Quote:
    return Quote(rate=Decimal("1.00"), label=bug.LABELS.get(ccy, "?")[:8])
