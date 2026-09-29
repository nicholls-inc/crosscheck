"""Round 7: an annotation naming a type imported from an unmodelled package
(`opentelemetry.util.types.Attributes` is `Optional[Mapping[...]]`) says
nothing about None: no non-null contract on parameters, returns or fields,
and a warning where a value of that type reaches a non-null requirement.
Final verification min/al1.
"""

from dataclasses import dataclass
from typing import Optional

from opentelemetry.util.types import Attributes
from pydantic import BaseModel, Field

import thirdparty.types as tt


class Span(BaseModel):
    name: str = Field(max_length=20)
    attributes: dict


def default_attributes(key: str) -> Attributes:
    # Not a violation of `-> Attributes` (it allows None).
    return None if key == "" else {"k": key}


def span(key: str) -> Span:
    # BUG: default_attributes may return None.
    return Span(name="op", attributes=default_attributes(key))


def lookup_attributes(source) -> Attributes:
    return source.attributes


def span_from(source) -> Span:
    # Unknown (warning): an `Attributes` value may be None.
    return Span(name="op", attributes=lookup_attributes(source))


def label(value: tt.Label = None) -> Span:
    # BUG: the `= None` default still makes the parameter nullable.
    return Span(name=value, attributes={})


@dataclass
class Event:
    attributes: Attributes
    note: Optional[str] = None


def emit() -> Event:
    # An external-typed field has no non-null requirement.
    return Event(attributes=None)
