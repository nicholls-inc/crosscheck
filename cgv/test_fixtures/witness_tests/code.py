"""CG-1.37: one error of each counterexample kind on a Django field and on a
pydantic field, a fractional counterexample against an integral field of each
library, and an error on a data class field, which gets no witness module."""

from decimal import Decimal

from models import Account
from records import Draft, Order


def open_account() -> Account:
    # BUG: every value breaks its field
    return Account.objects.create(
        code="abcdefgh",
        rate=Decimal("1.234"),
        level=11,
        points=-1,
        status="z",
        tier=3,
        note=None,
        score=Decimal("5.5"),
    )


def place_order() -> Order:
    # BUG: every value breaks its field
    return Order(
        ref="abcdef",
        amount=Decimal("0.125"),
        qty=12,
        discount=-0.5,
        channel="mail",
        label=None,
        count=Decimal("5.5"),
    )


def draft() -> Draft:
    # BUG: None into a non-Optional data class field; nothing checks it at run time
    return Draft(title=None)
