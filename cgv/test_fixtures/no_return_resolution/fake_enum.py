from other_lib import Enum


class Color(Enum):
    RED = 1
    GREEN = 2


def color_name(c: Color) -> str:
    match c:
        case Color.RED:
            return "red"
        case Color.GREEN:
            return "green"
