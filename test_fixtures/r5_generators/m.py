from dataclasses import dataclass
from decimal import Decimal
from typing import Iterator


@dataclass
class Batch:
    rows: object
    first: Decimal


def gen_bare(xs: list):
    # A bare `return` in a generator ends the iteration; the call returns a generator.
    if not xs:
        return
    yield 1


def gen_plain(xs: list):
    for x in xs:
        yield x


def gen_annot(xs: list) -> Iterator[int]:
    if not xs:
        return
    yield 1


def gen_from(xs: list):
    yield from xs


def first_amount(xs: list):
    # BUG: None for an empty list (not a generator)
    for x in xs:
        return Decimal(x)
    return None


def a(xs: list) -> Batch:
    return Batch(rows=gen_bare(xs), first=Decimal("1"))


def b(xs: list) -> Batch:
    return Batch(rows=gen_plain(xs), first=Decimal("1"))


def c(xs: list) -> Batch:
    return Batch(rows=gen_annot(xs), first=Decimal("1"))


def d(xs: list) -> Batch:
    return Batch(rows=gen_from(xs), first=Decimal("1"))


def e(xs: list) -> Batch:
    return Batch(rows=list(xs), first=first_amount(xs))
