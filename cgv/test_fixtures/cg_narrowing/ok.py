from typing import Optional

from records import Holder, Rec, clear, lookup


# Pattern 1: reassignment after an early return, with the conversion in a
# `try` whose handler exits or rebinds.
def convert_in_try(raw: dict) -> Rec:
    v = None
    if raw:
        v = lookup(raw, "n")
    if not v:
        return Rec(n=0, s="none")
    try:
        v = int(v)
    except ValueError:
        return Rec(n=0, s="bad")
    return Rec(n=v, s="ok")


def convert_in_try_default(raw: dict) -> Rec:
    v = lookup(raw, "n")
    if not v:
        return Rec(n=0, s="none")
    try:
        v = int(v)
    except ValueError:
        v = 0
    return Rec(n=v, s="ok")


def convert_in_try_else(raw: dict) -> Rec:
    v = lookup(raw, "n")
    try:
        n = int(raw["n"])
    except KeyError:
        return Rec(n=0, s="missing")
    else:
        v = "x" * n
    return Rec(n=n, s=v)


# Pattern 2: the caller guards an attribute, the callee reads it.
def label_of(h: Holder) -> Rec:
    return Rec(n=1, s=h.name)


def guarded_if(h: Holder) -> None:
    if h.name:
        label_of(h)


def guarded_return(h: Holder) -> None:
    if h.name is None:
        return
    label_of(h)


# Pattern 3: membership in a display of non-None literals.
def in_set(raw: dict) -> Rec:
    v = lookup(raw, "k")
    if v in {"a", "b"}:
        return Rec(n=1, s=v)
    return Rec(n=0, s="other")


def not_in_tuple(raw: dict) -> Rec:
    v = lookup(raw, "k")
    if v not in ("a", "b"):
        return Rec(n=0, s="other")
    return Rec(n=1, s=v)


def in_list_expr(raw: dict) -> Rec:
    v = lookup(raw, "k")
    return Rec(n=1, s=v if v in ["a", "b"] else "other")


# Pattern 4: `k in d` before `d.get(k)`.
def key_in(d: dict, k: str) -> Rec:
    if k in d:
        return Rec(n=1, s=d.get(k))
    return Rec(n=0, s="missing")


def literal_key_in(d: dict) -> Rec:
    if "name" in d:
        v = d.get("name")
        return Rec(n=1, s=v)
    return Rec(n=0, s="missing")


def key_not_in(d: dict, k: str) -> Rec:
    if k not in d:
        return Rec(n=0, s="missing")
    return Rec(n=1, s=d.get(k))


# Pattern 2: the read comes before the call that may write the field.
def label_then_clear(h: Holder) -> Rec:
    r = Rec(n=1, s=h.name)
    clear(h)
    return r


def guarded_then_clear(h: Holder) -> None:
    if h.name:
        label_then_clear(h)
