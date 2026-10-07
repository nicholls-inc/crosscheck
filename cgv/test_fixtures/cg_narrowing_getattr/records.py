"""CG-1.9: caller guards and functions a lookup by string can reach."""

from dataclasses import dataclass
from typing import Optional


@dataclass
class Rec:
    n: int
    s: str


@dataclass
class Holder:
    name: Optional[str]
