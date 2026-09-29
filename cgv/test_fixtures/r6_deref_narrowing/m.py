"""Round 6: after `x.attr`, `x.method(...)`, `x[i]` or `len(x)` runs, `x` is
not None for the rest of the block (it would have raised). Repro: final
pass min/fp1 (dispatch S06, S17).
"""

from typing import Optional

from pydantic import BaseModel


class Event(BaseModel):
    source: str


class Session:
    def add(self, o) -> None: ...


def log_event(db: Session, source: str) -> Event:
    return Event(source=source)


def close_flow(db=None) -> None:
    # SAFE: db.add would have raised on None
    db.add("x")
    log_event(db, "core")


def lookup(d: dict, k: str) -> Optional[str]:
    return d.get(k)


def fetch(d: dict, k: str) -> str:
    # SAFE: v.upper() would have raised on None
    v = lookup(d, k)
    v.upper()
    return v


def by_index(d: dict, k: str) -> Event:
    v = lookup(d, k)
    v[0]
    return Event(source=v)


def by_len(d: dict, k: str) -> Event:
    v = lookup(d, k)
    if len(v) > 3:
        pass
    return Event(source=v)


def use(d: dict) -> Event:
    return log_event(Session(), fetch(d, "a"))


def branch_only(d: dict, k: str, c: bool) -> Event:
    # BUG: the dereference happens on one branch only
    v = lookup(d, k)
    if c:
        v.upper()
    return Event(source=v)


def short_circuit(d: dict, k: str) -> Event:
    # BUG: `v and v.strip()` does not dereference None
    v = lookup(d, k)
    v and v.strip()
    return Event(source=v)


def dunder(d: dict, k: str) -> Event:
    # BUG: None has `__class__`
    v = lookup(d, k)
    v.__class__
    return Event(source=v)
