from models import Meter


def adjust_reading(x: int) -> int:
    """ensures: result >= -10
    ensures: result <= 100
    """
    return x - 10


def bump_count(x: int) -> int:
    """ensures: result >= 0"""
    return x + 1


def make_meter(x: int, y: int) -> Meter:
    return Meter.objects.create(reading=adjust_reading(x), count=bump_count(y))
