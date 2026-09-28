"""Correct code: 2-place amounts through the same dict forms."""

from decimal import Decimal

from bug import Ser, create
from models import Payment


def two(x: Decimal) -> Decimal:
    return x.quantize(Decimal("0.01"))


def payload_ok(x: Decimal) -> dict:
    return {"amount": two(x)}


def local_ok(amount: Decimal) -> Payment:
    data = {"amount": two(amount)}
    return create(data)


def method_ok(amount: Decimal) -> Payment:
    return Ser().create({"amount": two(amount)})


def returned_ok(x: Decimal) -> Payment:
    return Payment.objects.create(**payload_ok(x))


def dict_call_ok(x: Decimal) -> Payment:
    kwargs = dict(amount=two(x))
    return Payment.objects.create(**kwargs)
