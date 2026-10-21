"""CG-1.14: isinstance tests that CGV does not read yet."""

from models import Entry, Payload, Text


def store_project_class(value: object) -> Entry:
    # FALSE ERROR, NOT YET REACHED: a project class may be a Protocol, an ABC or have a
    # metaclass with __instancecheck__, so None may pass the test.
    if isinstance(value, Text):
        return Entry.objects.create(text=value)
    return Entry.objects.create(text="x")


def store_chained(payload: Payload) -> Entry:
    # FALSE ERROR, NOT YET REACHED: isinstance narrows a name only. A call, or code that
    # holds the payload or its address under another name, can set the postcode to None
    # between the test and the read.
    if isinstance(payload.address.postcode, str):
        return Entry.objects.create(text=payload.address.postcode)
    return Entry.objects.create(text="x")
