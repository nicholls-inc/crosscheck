from decimal import Decimal

from models import Grandchild, Payment, Precise, Restaurant


def four(x: Decimal) -> Decimal:
    return x.quantize(Decimal("0.0001"))


def two(x: Decimal) -> Decimal:
    return x.quantize(Decimal("0.01"))


def pay_bad(x: Decimal) -> Payment:
    # BUG: inherited abstract field `amount` is 2dp
    return Payment.objects.create(amount=four(x), fee=two(x))


def restaurant_bad() -> Restaurant:
    # BUG: `name` (concrete parent, max_length=5) gets 9 characters
    return Restaurant.objects.create(name="Chez Nous", seats=40)


def precise_ok(x: Decimal) -> Precise:
    # SAFE: Precise.amount is redefined with 4dp
    return Precise.objects.create(amount=four(x))


def grandchild_bad(x: Decimal) -> Grandchild:
    # BUG: two levels up
    g = Grandchild(fee=two(x), note="ok")
    g.amount = four(x)
    g.save()
    return g
