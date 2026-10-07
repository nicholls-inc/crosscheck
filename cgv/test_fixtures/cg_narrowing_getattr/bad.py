import sys

from records import Holder, Rec


# `getattr` with a computed name can reach any function, so no caller
# guard holds anywhere in the project.
def label_of(h: Holder) -> Rec:
    return Rec(n=1, s=h.name)


def guarded(h: Holder) -> None:
    if h.name:
        label_of(h)


def dispatch(name: str, h: Holder) -> None:
    getattr(sys.modules[__name__], name)(h)
