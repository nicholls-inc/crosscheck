from decimal import Decimal
from models import Payment


def four(x: Decimal) -> Decimal:
    return x.quantize(Decimal("0.0001"))


def pay(x: Decimal) -> Payment:
    return Payment.objects.create(amount=four(x))
