from enum import Enum, member as m


class Shade(Enum):
    DARK = 1

    @m
    def LIGHT(self):
        return 2


def shade_name(s: Shade) -> str:
    match s:
        case Shade.DARK:
            return "dark"


def const(f):
    return 3


class Tone(Enum):
    LOW = 1

    @const
    def HIGH(self):
        return 2


def tone_name(t: Tone) -> str:
    match t:
        case Tone.LOW:
            return "low"
