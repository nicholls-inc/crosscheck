"""Correct code: no errors."""

import abc
from decimal import Decimal
from typing import Protocol, TypeVar

from models import Reading

T = TypeVar("T")


def parse_kwh_ok(raw: str) -> Decimal:
    if not raw:
        raise ValueError("empty")
    return Decimal(raw).quantize(Decimal("0.001"))


def label_ok(code: str) -> str:
    if code:
        return code[:10]
    return "none"


class Source(Protocol):
    def name(self) -> str: ...


class Base(abc.ABC):
    @abc.abstractmethod
    def code(self) -> str:
        """A stub: not a return site."""


def first(xs: list[T]) -> T:
    # A type variable may stand for an Optional type: no contract.
    return xs[0] if xs else None


def store_ok(raw: str) -> Reading:
    return Reading.objects.create(source=label_ok(raw), kwh=parse_kwh_ok(raw))


def get_reading(pk: int) -> Reading:
    # `objects.get` raises rather than returning None.
    return Reading.objects.get(pk=pk)
