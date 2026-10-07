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


def lookup(raw: dict, k: str) -> Optional[str]:
    return raw.get(k)
