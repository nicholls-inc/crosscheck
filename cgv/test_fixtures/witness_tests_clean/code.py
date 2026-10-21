"""CG-1.37: the models of witness_tests with values every field accepts, so
`--witness-tests` writes no module."""

from decimal import Decimal

from models import Account
from records import Draft, Order


def open_account() -> Account:
    # SAFE
    return Account.objects.create(
        code="abc",
        rate=Decimal("1.23"),
        level=10,
        points=0,
        status="a",
        tier=2,
        note="n",
        score=5,
    )


def place_order() -> Order:
    # SAFE
    return Order(ref="ab", amount=Decimal("0.12"), qty=10, discount=0.5, channel="web", label="l", count=5)


def draft() -> Draft:
    # SAFE
    return Draft(title="t")
