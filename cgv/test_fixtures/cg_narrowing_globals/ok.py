from records import Holder, Rec


# This module looks nothing up by string, and `bad.py` does not import it.
def title_of(h: Holder) -> Rec:
    return Rec(n=2, s=h.name)


def guarded_title(h: Holder) -> None:
    if h.name:
        title_of(h)
