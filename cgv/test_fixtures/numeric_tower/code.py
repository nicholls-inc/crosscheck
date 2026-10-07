from dataclasses import dataclass
from decimal import Decimal


@dataclass
class Sample:
    level: float
    count: int


def to_decimal(x: Decimal) -> Decimal:
    return x


def to_text(x: str) -> str:
    return x


def decimal_into_float(x: Decimal) -> Sample:
    # BUG: Decimal is not part of the numeric tower
    return Sample(level=to_decimal(x), count=1)


def str_into_float(x: str) -> Sample:
    # BUG: a str is not a float
    return Sample(level=to_text(x), count=1)


def float_into_int(x: float) -> Sample:
    # BUG: the tower widens int to float, never float to int
    return Sample(level=1.0, count=x)

