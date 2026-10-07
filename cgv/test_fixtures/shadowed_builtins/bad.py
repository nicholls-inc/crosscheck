from dataclasses import dataclass

from records import Reading, measured
from units import float


class int:
    """A project type, defined here, that shadows the builtin int."""

    def __init__(self, raw: str) -> None:
        self.raw = raw


@dataclass
class Box:
    value: float


def parsed() -> float:
    return float("1.5")


def counted() -> int:
    return int("3")


def store_parsed() -> Reading:
    # BUG: units.float is not a float
    return Reading(value=parsed(), count=3)


def store_counted() -> Reading:
    # BUG: bad.int is not an int
    return Reading(value=1.5, count=counted())


def store_counted_value() -> Reading:
    # BUG: bad.int is not an int, so the numeric tower (an int where a float
    # is required) does not apply to it
    return Reading(value=counted(), count=3)


def box_measured() -> Box:
    # BUG: a builtin float is not a units.float
    return Box(value=measured())
