"""Round 6: a dict bound to a local (`{...}` or `dict(k=v)`), or returned by a
function whose returns are dict displays, carries its keys to `**` splats,
also through a function that splats its parameter. Final pass min/f10a-g,
min/u2 (a, e).
"""

from decimal import Decimal

from models import Payment


def three(x: Decimal) -> Decimal:
    return x.quantize(Decimal("0.001"))


def create(validated: dict) -> Payment:
    return Payment.objects.create(**validated)


class Ser:
    def create(self, validated):
        return Payment.objects.create(**validated)


def local_to_forwarder(amount: Decimal) -> Payment:
    # BUG (f10f): a local dict passed to a function that splats it
    data = {"amount": three(amount)}
    return create(data)


def local_to_method(amount: Decimal) -> Payment:
    # BUG (f10a): the same through a method
    ser = Ser()
    data = {"amount": amount.quantize(Decimal("0.001"))}
    return ser.create(data)


def payload(x: Decimal) -> dict:
    return {"amount": x.quantize(Decimal("0.001"))}


def returned_dict(x: Decimal) -> Payment:
    # BUG (u2 a): the keys of a returned dict display
    return Payment.objects.create(**payload(x))


def dict_call(x: Decimal) -> Payment:
    # BUG (u2 e): dict(k=v) bound to a local
    kwargs = dict(amount=x.quantize(Decimal("0.001")))
    return Payment.objects.create(**kwargs)


def returned_to_forwarder(x: Decimal) -> Payment:
    # BUG: a returned dict passed to a function that splats it
    return create(payload(x))
