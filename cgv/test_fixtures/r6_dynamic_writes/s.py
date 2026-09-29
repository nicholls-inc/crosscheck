"""Round 6: a write whose fields cannot be determined, on an object of a
known class, is an edge without guarantees to every field of the class
(a warning where the field has a requirement). Final pass min/u2.
"""

from decimal import Decimal

from models import Payment


def payload(x: Decimal) -> dict:
    return {"amount": x.quantize(Decimal("0.001"))}


def by_setattr(p: Payment, x: Decimal) -> None:
    # WARNING: setattr with a computed name
    for k, v in payload(x).items():
        setattr(p, k, v)
    p.save()


def by_dict_update(x: Decimal) -> Payment:
    # BUG: `__dict__.update(amount=...)` writes amount (3 places)
    p = Payment(amount=Decimal("0.00"))
    p.__dict__.update(amount=x.quantize(Decimal("0.001")))
    p.save()
    return p


def by_unknown_dict(data: dict) -> Payment:
    # WARNING: `**data` may hold any field
    return Payment.objects.create(**data)


def forwarder(**fields) -> Payment:
    # No warning here: the keywords are checked at each call (`kw_forwards`).
    return Payment.objects.create(**fields)


def via_forwarder(x: Decimal) -> Payment:
    # BUG: 3 places through the forwarder
    return forwarder(amount=x.quantize(Decimal("0.001")))
