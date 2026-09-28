"""Round 7: `.get(k, default)` on a module-level dict whose values are all
non-None literals, never mutated in the project, is one of its values or the
default. Adversarial p30, n03.
"""

from decimal import Decimal
from typing import Optional

from pydantic import BaseModel, Field

RATES = {"EUR": Decimal("1.0845"), "USD": Decimal("1.2711")}
LABELS = {"EUR": "Euro", "USD": "US dollar"}
OVERRIDES = {"EUR": "Euro"}


class Quote(BaseModel):
    rate: Decimal = Field(decimal_places=2)
    label: str = Field(max_length=8)


def rate_for(ccy: str) -> Optional[Decimal]:
    return RATES.get(ccy)


def quote(ccy: str) -> Quote:
    # BUG: rate_for may return None (and has 4 places)
    return Quote(rate=rate_for(ccy), label="x")


def quote_default(ccy: str) -> Quote:
    # BUG: never None, but 4 places; "US dollar" has 9 characters
    return Quote(rate=RATES.get(ccy, Decimal("1")), label=LABELS.get(ccy, "?"))


def set_override(ccy: str, label: str) -> None:
    OVERRIDES[ccy] = label


def quote_override(ccy: str) -> Quote:
    # Unknown (warning): OVERRIDES is mutated, so its values are not known.
    return Quote(rate=Decimal("1.00"), label=OVERRIDES.get(ccy, "?"))
