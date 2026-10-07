from records import Holder, Rec, clear, lookup


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


# Pattern 4: a handler removes the key and exits, and `finally` still runs.
def key_popped_before_finally(d: dict, k: str) -> Rec:
    if k in d:
        try:
            x = 1
        except ValueError:
            d.pop(k)
            raise
        finally:
            return Rec(n=1, s=d.get(k))
    return Rec(n=0, s="missing")


# Pattern 4: a class body removes the key when it is defined.
def key_popped_in_class(d: dict, k: str) -> Rec:
    if k in d:
        class Gone:
            x = d.pop(k)
        return Rec(n=1, s=d.get(k))
    return Rec(n=0, s="missing")


# Pattern 4: another task may remove the key while this one is suspended.
async def key_popped_across_await(d: dict, k: str, other) -> Rec:
    if k in d:
        await other
        return Rec(n=1, s=d.get(k))
    return Rec(n=0, s="missing")


# Pattern 4: an async comprehension suspends on every item.
async def key_popped_across_async_comp(d: dict, k: str, it) -> Rec:
    if k in d:
        xs = [x async for x in it]
        return Rec(n=1, s=d.get(k))
    return Rec(n=0, s="missing")


# Pattern 4: `o` is not known to be a dict, so `o.get(k)` need not be `o[k]`.
def key_in_non_dict(o, k: str) -> Rec:
    if k in o:
        return Rec(n=1, s=o.get(k))
    return Rec(n=0, s="missing")


# Pattern 4: another object's `.get` may remove the key (a cache over `d`).
def key_after_other_get(d: dict, cache, k: str) -> Rec:
    if k in d:
        cache.get(k)
        return Rec(n=1, s=d.get(k))
    return Rec(n=0, s="missing")


# Pattern 4: the key is removed inside the expression, after the test.
def key_popped_in_and(d: dict, k: str) -> Rec:
    if k in d:
        return Rec(n=1, s=str(d.pop(k)) and d.get(k))
    return Rec(n=0, s="missing")


def key_popped_in_conditional(d: dict, k: str) -> Rec:
    if k in d:
        return Rec(n=1, s=d.get(k) if d.pop(k) else "x")
    return Rec(n=0, s="missing")


# Pattern 4: a filter removes the key before the element is built.
def key_popped_in_comprehension(d: dict, k: str, ks: list) -> list:
    if k in d:
        return [Rec(n=1, s=d.get(k)) for _ in ks if d.pop(k)]
    return []


# Pattern 4: a filter's `k in d` is not carried to the element.
def key_checked_in_filter(d: dict, ks: list) -> list:
    return [Rec(n=1, s=d.get(k)) for k in ks if k in d if d.pop(k)]


# Pattern 2: the callee hands `h` to code that may write `h.name` before
# the read: a call with `h`, a method of `h`, a call after `h` is aliased.
def name_after_clear(h: Holder) -> Rec:
    clear(h)
    return Rec(n=5, s=h.name)


def name_after_forget(h: Holder) -> Rec:
    h.forget()
    return Rec(n=6, s=h.name)


def name_after_alias(h: Holder) -> Rec:
    q = h
    clear(q)
    return Rec(n=7, s=h.name)


def call_writers(h: Holder) -> None:
    if h.name:
        name_after_clear(h)
        name_after_forget(h)
        name_after_alias(h)
