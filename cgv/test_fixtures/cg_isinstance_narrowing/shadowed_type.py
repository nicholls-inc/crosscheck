"""CG-1.14: a module that binds a type name gets no narrowing on that name."""

from models import Entry

str = type(None)


def store_shadowed_type(value: object) -> Entry:
    # BUG: `str` is NoneType in this module.
    if isinstance(value, str):
        return Entry.objects.create(text=value)
    return Entry.objects.create(text="x")
