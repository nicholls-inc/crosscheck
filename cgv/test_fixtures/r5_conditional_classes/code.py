from decimal import Decimal

from models import Fee, Point, Price


def bad(x: Decimal) -> Price:
    # BUG: 4 decimal places into a 2dp column of a conditionally defined model
    return Price.objects.create(amount=x.quantize(Decimal("0.0001")))


def bad_fee(x: Decimal) -> Fee:
    # BUG: the same for a model defined in a `try` block
    return Fee.objects.create(amount=x.quantize(Decimal("0.001")))
