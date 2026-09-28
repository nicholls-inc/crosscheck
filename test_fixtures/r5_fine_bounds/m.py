from decimal import Decimal

from pydantic import BaseModel, Field

# Bounds with 7 decimal places: exact only in the decimal columns (round 5;
# the micros columns round them by role). adversarial2 min/e2.
# Intended verdict; passes once the checker reads param_*_decimal.


class P(BaseModel):
    x: Decimal = Field(le=Decimal("0.1234567"))
    y: Decimal = Field(ge=Decimal("0.1234567"))


def exact() -> P:
    # SAFE: equal to the bounds
    return P(x=Decimal("0.1234567"), y=Decimal("0.1234567"))


def inside() -> P:
    return P(x=Decimal("0.12345669"), y=Decimal("1.5e-1"))


def bad() -> P:
    # BUG: one ten-millionth outside each bound
    return P(x=Decimal("0.1234568"), y=Decimal("0.1234566"))
