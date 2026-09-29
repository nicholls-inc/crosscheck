from decimal import Decimal

from m import Band


def rate_for(kind: str) -> Decimal:
    # SAFE: the wildcard raises (adversarial2 min/m15a)
    match kind:
        case "peak":
            return Decimal("0.35")
        case _:
            raise ValueError(kind)


def rate_default(kind: str) -> Decimal:
    # SAFE: every case returns (min/m15d)
    match kind:
        case "peak":
            return Decimal("0.35")
        case _:
            return Decimal("0.10")


def rate_capture(kind: str):
    # SAFE: a capture pattern and an or-pattern with a wildcard always match
    match kind:
        case "peak" | "night":
            return Decimal("0.35")
        case other:
            return Decimal("0.10")


def rate_or_wildcard(kind: str):
    match kind:
        case "peak":
            return Decimal("0.35")
        case "x" | _:
            return Decimal("0.10")


def make(kind: str) -> Band:
    return Band(rate=rate_for(kind))


def make_default(kind: str) -> Band:
    return Band(rate=rate_default(kind))


def make_capture(kind: str) -> Band:
    return Band(rate=rate_capture(kind))


def make_or(kind: str) -> Band:
    return Band(rate=rate_or_wildcard(kind))
