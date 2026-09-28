from typing import NamedTuple

import attr


class Coord(NamedTuple):
    label: str
    value: int


@attr.s
class Point:
    label: str = attr.ib()
    value: int = attr.ib()
