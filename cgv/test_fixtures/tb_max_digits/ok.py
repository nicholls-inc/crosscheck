from decimal import Decimal

from models import Price, Quote


def record_largest() -> Price:
    # SAFE: 999.99 is the largest value with 3 integer digits and 2 places
    return Price.objects.create(amount=Decimal("999.99"))


def record_smallest() -> Price:
    # SAFE: -999.99 is the smallest
    return Price.objects.create(amount=Decimal("-999.99"))


def quote_largest() -> Quote:
    # SAFE: the same limit for pydantic
    return Quote(amount=Decimal("999.99"))
