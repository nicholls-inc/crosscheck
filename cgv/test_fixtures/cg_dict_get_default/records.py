"""CG-1.29: `d.get(k, default)` returns the stored value when `k` is a key,
whatever the default, so it is None when the dict stores None under `k`.
`bug.py` must give a non-null error on each read. `ok.py` must give none.
"""

from dataclasses import dataclass


@dataclass
class Payer:
    name: str
    city: str
