from dataclasses import dataclass
from typing import Optional


@dataclass
class Label:
    text: str


@dataclass
class Reading:
    note: Optional[str]


def relabel(r: Reading) -> Label:
    # BUG: an Optional field written into a non-null field
    return Label(text=r.note)
