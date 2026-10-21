"""CG-1.14: isinstance tests that rule out None narrow the subject."""

from typing import Optional

from models import Entry


def store_text(value: object) -> Entry:
    # NO ERROR: the branch runs only for a str.
    if isinstance(value, str):
        return Entry.objects.create(text=value)
    return Entry.objects.create(text="x")


def store_early_exit(value: object) -> Optional[Entry]:
    # NO ERROR: the function returns unless value is a str.
    if not isinstance(value, str):
        return None
    return Entry.objects.create(text=value)


def store_assert(value: object) -> Entry:
    # NO ERROR: the assert rules out None.
    assert isinstance(value, str)
    return Entry.objects.create(text=value)


def store_tuple(value: object) -> Entry:
    # NO ERROR: no member of the tuple admits None.
    if isinstance(value, (str, (int, float))):
        return Entry.objects.create(text=value)
    return Entry.objects.create(text="x")


def store_union(value: object) -> Entry:
    # NO ERROR: no member of the union admits None.
    if isinstance(value, str | bytes):
        return Entry.objects.create(text=value)
    return Entry.objects.create(text="x")


def store_and(value: object, flag: bool) -> Entry:
    # NO ERROR: both conjuncts hold in the branch.
    if isinstance(value, str) and flag:
        return Entry.objects.create(text=value)
    return Entry.objects.create(text="x")


def store_or_else(value: object, flag: bool) -> Entry:
    # NO ERROR: the else branch runs only when value is a str.
    if not isinstance(value, str) or flag:
        return Entry.objects.create(text="x")
    else:
        return Entry.objects.create(text=value)


def store_elif(value: object, flag: bool) -> Entry:
    # NO ERROR: the elif branch runs only for a str.
    if flag:
        return Entry.objects.create(text="x")
    elif isinstance(value, str):
        return Entry.objects.create(text=value)
    return Entry.objects.create(text="y")


def store_ternary(value: object) -> Entry:
    # NO ERROR: the true branch of the conditional runs only for a str.
    return Entry.objects.create(text=value if isinstance(value, str) else "")
