"""Round 6: a non-Optional return annotation is a contract on the function's
own return values, checked at each `return`; callers rely on it.
"""

from decimal import Decimal
from typing import Optional

from models import Reading


def parse_kwh(raw: str) -> Decimal:
    # BUG: returns None under `-> Decimal` (reported here, once)
    if not raw:
        return None
    return Decimal(raw).quantize(Decimal("0.001"))


def label(code: str) -> str:
    # BUG: falls off the end (an implicit `return None`) when code is empty
    if code:
        return code[:10]


def unit(code: str) -> str:
    # BUG: a bare `return` under `-> str`
    if code == "x":
        return
    return "kWh"


def as_text(n: int) -> str:
    # BUG: an int under `-> str`
    return n + 1


def store(raw: str) -> Reading:
    # SAFE for Reading.kwh: callers rely on parse_kwh's annotation (the
    # None return is parse_kwh's own error, above)
    return Reading.objects.create(source=label(raw), kwh=parse_kwh(raw))


def maybe(raw: str) -> Optional[Decimal]:
    return None if not raw else Decimal("1.000")


def store_maybe(raw: str) -> Reading:
    # BUG: an Optional result into a non-null field (the annotation allows None)
    return Reading.objects.create(source="m", kwh=maybe(raw))
