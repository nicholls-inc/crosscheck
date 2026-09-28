from dataclasses import replace
from decimal import Decimal

from m import Inv, PayoutSerializer, Row, Payout


def two(x: Decimal) -> Decimal:
    return x.quantize(Decimal("0.01"))


def a_ok(r: Row, d: dict) -> Row:
    return replace(r, name=d.get("x") or "anon")


def c_ok(i: Inv, x: Decimal) -> Inv:
    return i.model_copy(update={"total": two(x)})


def e_ok(x: Decimal) -> Inv:
    return Inv.model_validate({"total": two(x)})


def f_ok(x: Decimal) -> Inv:
    base = {"total": x.quantize(Decimal("0.0001"))}
    # SAFE: the explicit key overrides the 4dp entry of the spread
    return Inv(**{**base, "total": two(x)})


def f_overridden(x: Decimal, other: dict) -> Inv:
    # SAFE for this analysis: an unknown spread after the key may replace it
    return Inv(**{"total": x.quantize(Decimal("0.0001")), **other})


def g_ok(r: Row) -> None:
    setattr(r, "name", "fixed")


def payout_ok(x: Decimal) -> Payout:
    return PayoutSerializer().create({"reference": "PO-2", "fee": two(x)})
