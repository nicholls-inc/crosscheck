from typing import Optional

from records import Coord, Point


def get_label(n: int) -> Optional[str]:
    if n > 0:
        return "pos"
    return None


def make_coord(n: int, v: int) -> Coord:
    return Coord(get_label(n), v)  # bug: label may be None, Coord.label is non-null


def make_point(v: int) -> Point:
    return Point("fixed", v)  # literal, non-null field: no error either way
