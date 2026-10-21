"""CG-1.14: a module that binds `isinstance` gets no isinstance narrowing."""

from models import Entry


def isinstance(value, kind):
    return True


def store_shadowed(value: object) -> Entry:
    # BUG: this module's isinstance returns True for None.
    if isinstance(value, str):
        return Entry.objects.create(text=value)
    return Entry.objects.create(text="x")
