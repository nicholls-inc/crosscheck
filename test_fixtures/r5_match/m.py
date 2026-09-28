from decimal import Decimal

from pydantic import BaseModel, Field


class Band(BaseModel):
    rate: Decimal = Field(decimal_places=2)


def rate_partial(kind: str):
    # BUG: no wildcard case: falls through to None
    match kind:
        case "peak":
            return Decimal("0.35")
        case "off":
            return Decimal("0.10")


def rate_guarded(kind: str, strict: bool):
    # BUG: the wildcard is guarded, so it may not match
    match kind:
        case "peak":
            return Decimal("0.35")
        case _ if strict:
            raise ValueError(kind)


def rate_4dp(kind: str):
    # BUG: 4 decimal places (round 5 N1, adversarial2 n15)
    match kind:
        case "peak":
            return Decimal("0.3512")
        case _:
            return Decimal("0.12")


def make_partial(kind: str) -> Band:
    return Band(rate=rate_partial(kind))


def make_guarded(kind: str, strict: bool) -> Band:
    return Band(rate=rate_guarded(kind, strict))


def make_4dp(kind: str) -> Band:
    return Band(rate=rate_4dp(kind))
