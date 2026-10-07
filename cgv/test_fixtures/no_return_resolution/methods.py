from typing import NoReturn

from external_lib import Mixin


class Base:
    @staticmethod
    def s(msg: str) -> NoReturn:
        raise ValueError(msg)


class ViaUnreadable(Mixin, Base):
    pass


class ViaAssignment(Base):
    s = staticmethod(print)


class Left(Base):
    pass


class Right(Base):
    @staticmethod
    def s(msg: str) -> None:
        return None


class Diamond(Left, Right):
    pass


class Rebound:
    @staticmethod
    def s(msg: str) -> NoReturn:
        raise ValueError(msg)

    s = staticmethod(print)


def unreadable_base(kind: str) -> str:
    if kind == "a":
        return "first"
    ViaUnreadable.s(kind)


def class_body_assignment(kind: str) -> str:
    if kind == "a":
        return "first"
    ViaAssignment.s(kind)


def diamond(kind: str) -> str:
    if kind == "a":
        return "first"
    Diamond.s(kind)


def rebound_in_body(kind: str) -> str:
    if kind == "a":
        return "first"
    Rebound.s(kind)
