import enum
from enum import Enum

from django.db import models

from methods import Base, Left


class Plain(Base):
    pass


class Grand(Left, object):
    pass


def through_subclass(kind: str) -> str:
    if kind == "a":
        return "first"
    Plain.s(kind)


def through_grandchild(kind: str) -> str:
    if kind == "a":
        return "first"
    Grand.s(kind)


def through_base(kind: str) -> str:
    if kind == "a":
        return "first"
    Base.s(kind)


class Size(str, Enum):
    SMALL = "s"
    LARGE = "l"

    @staticmethod
    def default() -> str:
        return "s"

    @classmethod
    def names(cls) -> list:
        return []

    @property
    def upper(self) -> str:
        return "S"


def size_name(s: Size) -> str:
    match s:
        case Size.SMALL:
            return "small"
        case Size.LARGE:
            return "large"


class Level(enum.IntEnum):
    LOW = 1
    HIGH = 2


def level_name(level: Level) -> str:
    match level:
        case Level.LOW | Level.HIGH:
            return "level"


class Kind(models.TextChoices):
    A = "a", "A"
    B = "b", "B"


def kind_name(k: Kind) -> str:
    match k:
        case Kind.A:
            return "a"
        case Kind.B:
            return "b"
