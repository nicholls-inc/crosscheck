from typing import assert_never

from models import Customer, Entry, Marker, Sample


def count_visits(customer: Customer) -> int:
    return customer.visits


def weigh(points: float) -> Sample:
    return Sample(value=points)


def score_customer(customer: Customer) -> Sample:
    # FALSE POSITIVE (numeric-tower): an int is a valid float
    return weigh(count_visits(customer))


def remember(value: object) -> Entry:
    return Entry.objects.create(text=str(value))


def remember_nickname(customer: Customer) -> Entry:
    # FALSE POSITIVE (object-param): `object` accepts None and str() never
    # returns None
    return remember(customer.nickname)


def describe(kind: str) -> str:
    # FALSE POSITIVE (no-return): every case returns or hits assert_never
    match kind:
        case "a":
            return "first"
        case "b":
            return "second"
        case _:
            assert_never(kind)


def parse_level(raw: str) -> Marker:
    # FALSE POSITIVE (narrowing): level is assigned before it is used, the
    # except branch returns
    level = None
    try:
        level = int(raw)
    except ValueError:
        return Marker.objects.create(code="bad", level=0)
    return Marker.objects.create(code=raw, level=level)
