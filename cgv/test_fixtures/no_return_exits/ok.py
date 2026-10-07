import os
import sys
import typing
from typing import Never, NoReturn, Optional, assert_never

from django.db.models import TextChoices

from models import Color, Report


class Status(TextChoices):
    OPEN = "open", "Open"
    CLOSED = "closed", "Closed"


def fail(msg: str) -> NoReturn:
    raise ValueError(msg)


def stop(msg: str) -> Never:
    raise ValueError(msg)


async def fail_later(msg: str) -> NoReturn:
    raise ValueError(msg)


def describe(kind: str) -> str:
    match kind:
        case "a":
            return "first"
        case _:
            assert_never(kind)


def describe_typing(kind: str) -> str:
    if kind == "a":
        return "first"
    typing.assert_never(kind)


def label_sys_exit(kind: str) -> str:
    if kind == "a":
        return "first"
    sys.exit(1)


def label_os_exit(kind: str) -> str:
    if kind == "a":
        return "first"
    os._exit(1)


def label_abort(kind: str) -> str:
    if kind == "a":
        return "first"
    os.abort()


def label_builtin_exit(kind: str) -> str:
    if kind == "a":
        return "first"
    exit(1)


def label_quit(kind: str) -> str:
    if kind == "a":
        return "first"
    quit()


def label_fail(kind: str) -> str:
    if kind == "a":
        return "first"
    fail(kind)


def label_stop(kind: str) -> str:
    if kind == "a":
        return "first"
    stop(kind)


async def label_async(kind: str) -> str:
    if kind == "a":
        return "first"
    await fail_later(kind)


def report_for(name: Optional[str]) -> Report:
    if name is None:
        sys.exit(1)
    return Report(label=name)


def color_name(c: Color) -> str:
    match c:
        case Color.RED | Color.GREEN:
            return "warm"
        case Color.BLUE:
            return "cool"


def status_name(s: Status) -> str:
    match s:
        case Status.OPEN:
            return "open"
        case Status.CLOSED as closed:
            return "closed"
