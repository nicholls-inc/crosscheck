"""Correct code: `Attributes` may be None, and the code allows for it."""

from typing import Optional

from opentelemetry.util.types import Attributes

MaybeStr = Optional[str]


def ext(attributes: Attributes) -> dict:
    return dict(attributes or {})


def loc(name: MaybeStr) -> str:
    return name or "anon"


def record(attributes: Attributes = None, name: MaybeStr = None) -> None:
    ext(attributes)
    loc(name)


def forward(attributes: Attributes) -> None:
    ext(attributes)
