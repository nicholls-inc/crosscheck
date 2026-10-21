"""CG-1.14: an isinstance test that does not rule out None narrows nothing."""

from typing import Optional

from models import Entry, Payload


def store_negated(value: object) -> Entry:
    # BUG: the branch runs when value is not a str, None included.
    if not isinstance(value, str):
        return Entry.objects.create(text=value)
    return Entry.objects.create(text="x")


def store_none_type(value: object) -> Entry:
    # BUG: the tuple names NoneType, so None passes the test.
    if isinstance(value, (str, type(None))):
        return Entry.objects.create(text=value)
    return Entry.objects.create(text="x")


def store_object(value: object) -> Entry:
    # BUG: None is an instance of object.
    if isinstance(value, object):
        return Entry.objects.create(text=value)
    return Entry.objects.create(text="x")


def store_union_none(value: object) -> Entry:
    # BUG: `str | None` admits None.
    if isinstance(value, str | None):
        return Entry.objects.create(text=value)
    return Entry.objects.create(text="x")


def store_or(value: object, flag: bool) -> Entry:
    # BUG: the branch also runs when flag is true and value is None.
    if isinstance(value, str) or flag:
        return Entry.objects.create(text=value)
    return Entry.objects.create(text="x")


def store_param_shadow(value: object, str: type) -> Entry:
    # BUG: `str` is the parameter here, which may be NoneType.
    if isinstance(value, str):
        return Entry.objects.create(text=value)
    return Entry.objects.create(text="x")


def store_local_isinstance(value: object) -> Entry:
    # BUG: `isinstance` is a local function here, which may return True for None.
    isinstance = lambda v, t: True
    if isinstance(value, str):
        return Entry.objects.create(text=value)
    return Entry.objects.create(text="x")


def store_other_name(value: object, other: object) -> Entry:
    # BUG: the test is on another name.
    if isinstance(other, str):
        return Entry.objects.create(text=value)
    return Entry.objects.create(text="x")


def store_rebound(value: object, fallback: Optional[str]) -> Entry:
    # BUG: value is rebound to an Optional after the test.
    if isinstance(value, str):
        value = fallback
        return Entry.objects.create(text=value)
    return Entry.objects.create(text="x")


def store_chain_unguarded(payload: Payload) -> Entry:
    # BUG: the test is on the address, not on the postcode.
    if isinstance(payload.address, object):
        return Entry.objects.create(text=payload.address.postcode)
    return Entry.objects.create(text="x")
