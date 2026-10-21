"""CG-1.14: isinstance tests that CGV does not read yet."""

from models import Entry, Payload, Text, clear


def store_project_class(value: object) -> Entry:
    # FALSE ERROR, NOT YET REACHED: a project class may be a Protocol, an ABC or have a
    # metaclass with __instancecheck__, so None may pass the test.
    if isinstance(value, Text):
        return Entry.objects.create(text=value)
    return Entry.objects.create(text="x")


def store_chain_handed(payload: Payload) -> Entry:
    # MISSED ERROR, NOT YET REACHED: clear() sets the postcode to None after the
    # test. A narrowing within one function survives a call that is handed the
    # object, as `if payload.address.postcode is not None:` does (see the README).
    if isinstance(payload.address.postcode, str):
        clear(payload.address)
        return Entry.objects.create(text=payload.address.postcode)
    return Entry.objects.create(text="x")
