from decimal import Decimal

from models import Price


def four(x: Decimal) -> Decimal:
    return x.quantize(Decimal("0.0001"))


def create_price(**fields) -> Price:
    return Price.objects.create(**fields)


def caller(x: Decimal) -> Price:
    return create_price(value=four(x))
