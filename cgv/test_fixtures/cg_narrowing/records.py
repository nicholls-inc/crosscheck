"""CG-1.9: four narrowing patterns that gave false errors on a real
codebase (issue #5). `ok.py` holds the safe forms and must pass. `bugs.py`
holds near misses that must stay errors.
"""

from dataclasses import dataclass
from typing import Optional


@dataclass
class Rec:
    n: int
    s: str


@dataclass
class Holder:
    name: Optional[str]

    def forget(self) -> None:
        self.name = None


def clear(h: Holder) -> None:
    h.name = None


def lookup(raw: dict, k: str) -> Optional[str]:
    return raw.get(k)
