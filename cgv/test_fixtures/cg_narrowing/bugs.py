from records import Holder, Rec, lookup


# Pattern 1: the handler falls through without rebinding, so `v` may still
# be None.
def try_handler_passes(raw: dict) -> Rec:
    v = None
    try:
        v = int(raw["n"])
    except KeyError:
        pass
    return Rec(n=v, s="x")


# Pattern 2: one caller does not guard.
def tag_of(h: Holder) -> Rec:
    return Rec(n=2, s=h.name)


def tag_guarded(h: Holder) -> None:
    if h.name:
        tag_of(h)


def tag_unguarded(h: Holder) -> None:
    tag_of(h)


# Pattern 2: every call guards, but the function escapes as a value, so
# callers the project cannot see may pass anything.
def title_of(h: Holder) -> Rec:
    return Rec(n=3, s=h.name)


def title_guarded(h: Holder) -> None:
    if h.name:
        title_of(h)


HANDLERS = [title_of]


# Pattern 3: None is one of the literals.
def in_set_with_none(raw: dict) -> Rec:
    v = lookup(raw, "k")
    if v in {"a", None}:
        return Rec(n=1, s=v)
    return Rec(n=0, s="other")


# Pattern 4: the key is removed, or another dict is read, after the check.
def key_popped(d: dict, k: str) -> Rec:
    if k in d:
        d.pop(k)
        return Rec(n=1, s=d.get(k))
    return Rec(n=0, s="missing")


def other_dict(d: dict, e: dict, k: str) -> Rec:
    if k in d:
        return Rec(n=1, s=e.get(k))
    return Rec(n=0, s="missing")


def key_rebound(d: dict, k: str, j: str) -> Rec:
    if k in d:
        k = j
        return Rec(n=1, s=d.get(k))
    return Rec(n=0, s="missing")


# Pattern 4: the key is deleted after the check.
def key_deleted(d: dict, k: str) -> Rec:
    if k in d:
        del d[k]
        return Rec(n=1, s=d.get(k))
    return Rec(n=0, s="missing")


# Pattern 4: a handler runs after the body removed the key.
def key_popped_in_try(d: dict, k: str) -> Rec:
    if k in d:
        try:
            d.pop(k)
            check()
        except ValueError:
            return Rec(n=1, s=d.get(k))
    return Rec(n=0, s="missing")


# Pattern 4: a case body removes the key.
def key_popped_in_match(d: dict, k: str, mode: int) -> Rec:
    if k in d:
        match mode:
            case 1:
                d.pop(k)
        return Rec(n=1, s=d.get(k))
    return Rec(n=0, s="missing")


# Pattern 4: a walrus in a nested test rebinds the key.
def key_walrus(d: dict, k: str, j: str) -> Rec:
    if k in d:
        if (k := j):
            pass
        return Rec(n=1, s=d.get(k))
    return Rec(n=0, s="missing")


# Pattern 2: the only caller is the function itself.
def render(h: Holder, depth: int) -> Rec:
    if depth:
        if h.name:
            render(h, depth - 1)
    return Rec(n=4, s=h.name)
