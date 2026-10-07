from dataclasses import dataclass


@dataclass
class Reading:
    value: float
    count: int


def measured() -> float:
    return 1.5
