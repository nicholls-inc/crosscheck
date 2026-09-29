from records import Ticket


def safe_assert(o):
    label = getattr(o, "label", None)
    assert label is not None
    return label


def make_assert(o) -> Ticket:
    return Ticket(label=safe_assert(o))


def safe_truthy(o):
    label = getattr(o, "label", None)
    if label:
        return label
    return "default"


def make_truthy(o) -> Ticket:
    return Ticket(label=safe_truthy(o))


def unsafe(o):
    return getattr(o, "label", None)


def make_unsafe(o) -> Ticket:
    return Ticket(label=unsafe(o))  # bug: no narrowing, label may be None
