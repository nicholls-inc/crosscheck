from records import Holder, Rec


# `globals()` can reach any function of this module, so no caller guard
# holds here.
def label_of(h: Holder) -> Rec:
    return Rec(n=1, s=h.name)


def guarded(h: Holder) -> None:
    if h.name:
        label_of(h)


def dispatch(name: str, h: Holder) -> None:
    globals()[name](h)
