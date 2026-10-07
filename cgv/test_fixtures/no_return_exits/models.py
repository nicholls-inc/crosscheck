from dataclasses import dataclass
from enum import Enum, Flag, auto
from typing import NoReturn, Optional


@dataclass
class Report:
    label: str


class Color(Enum):
    RED = 1
    GREEN = 2
    BLUE = auto()


class Perm(Flag):
    READ = auto()
    WRITE = auto()


def logged(fn):
    def wrapper(*args, **kwargs):
        return fn(*args, **kwargs)

    return wrapper


@logged
def fail_logged(msg: str) -> NoReturn:
    raise ValueError(msg)


async def fail_async(msg: str) -> NoReturn:
    raise ValueError(msg)


def color_partial(c: Color) -> str:
    # BUG: BLUE matches no case, so the match falls through to None
    match c:
        case Color.RED:
            return "red"
        case Color.GREEN:
            return "green"


def perm_name(p: Perm) -> str:
    # BUG: a combined flag (READ | WRITE) matches no case
    match p:
        case Perm.READ:
            return "read"
        case Perm.WRITE:
            return "write"


def color_optional(c: Optional[Color]) -> str:
    # BUG: None matches no case
    match c:
        case Color.RED | Color.GREEN | Color.BLUE:
            return "color"


def color_rebound(c: Color, raw: str) -> str:
    # BUG: c no longer holds the argument
    c = raw
    match c:
        case Color.RED | Color.GREEN | Color.BLUE:
            return "color"


def report_logged(name: Optional[str]) -> Report:
    if name is None:
        # BUG: the decorator may return
        fail_logged("no name")
    return Report(label=name)


def label_unawaited(kind: str) -> str:
    if kind == "a":
        return "first"
    # BUG: without await this only creates a coroutine
    fail_async(kind)


def label_shadowed(kind: str) -> str:
    exit = print
    if kind == "a":
        return "first"
    # BUG: exit is print here
    exit(kind)
