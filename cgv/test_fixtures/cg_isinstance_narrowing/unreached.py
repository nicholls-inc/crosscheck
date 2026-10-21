"""CG-1.14: isinstance tests that CGV does not read yet."""

from models import Entry, Text


def store_project_class(value: object) -> Entry:
    # FALSE ERROR, NOT YET REACHED: a project class may be a Protocol, an ABC or have a
    # metaclass with __instancecheck__, so None may pass the test.
    if isinstance(value, Text):
        return Entry.objects.create(text=value)
    return Entry.objects.create(text="x")

