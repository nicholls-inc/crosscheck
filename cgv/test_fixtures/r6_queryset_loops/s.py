"""Round 6: iterating a queryset (or a typed collection, or a list later
passed to `bulk_update`) types the loop variable, so attribute writes on it
are writes. Final pass min/u2 (c).
"""

from decimal import Decimal

from models import Payment


def bulk(ps: list, x: Decimal) -> None:
    # BUG: the elements are Payments (bulk_update); 3 places
    for p in ps:
        p.amount = x.quantize(Decimal("0.001"))
    Payment.objects.bulk_update(ps, ["amount"])


def over_queryset(x: Decimal) -> None:
    # BUG: a 9-character ref on every payment of the queryset
    for p in Payment.objects.filter(amount__gt=0):
        p.ref = "REF-00001"
        p.save()


def over_typed(ps: list[Payment]) -> None:
    # BUG: a typed collection
    for p in ps:
        p.amount = Decimal("1.005")
        p.save()


def over_queryset_ok(x: Decimal) -> None:
    qs = Payment.objects.all()
    for p in qs:
        p.amount = x.quantize(Decimal("0.01"))
        p.ref = p.ref[:8]
    Payment.objects.bulk_update(qs, ["amount", "ref"])
