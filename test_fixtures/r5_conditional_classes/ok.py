from decimal import Decimal

from models import Fee, Point, Price


def good(x: Decimal) -> Price:
    return Price.objects.create(amount=x.quantize(Decimal("0.01")))


def good_fee(x: Decimal) -> Fee:
    return Fee.objects.create(amount=x.quantize(Decimal("0.01")))


def good_point() -> Point:
    return Point(x=2, label="p")
