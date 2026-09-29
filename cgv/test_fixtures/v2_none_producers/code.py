import re

from records import Bag, Sample


def pick_ternary(a, c):
    return a if c else None


def pick_matched(text):
    return re.match(r"\d+", text)


def pick_attr(o):
    return getattr(o, "name", None)


def pick_next(xs):
    return next(iter(xs), None)


def pick_latest():
    return Sample.objects.order_by("-id").values_list("amount", flat=True).first()


def assemble(a, c, text, o, xs) -> Bag:
    return Bag(
        ternary=pick_ternary(a, c),
        matched=pick_matched(text),
        attr=pick_attr(o),
        nxt=pick_next(xs),
        latest=pick_latest(),
    )
