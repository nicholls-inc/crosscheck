"""max_digits=5, decimal_places=2 accepts at most 3 integer digits, so the
largest magnitude is 999.99. Django's DecimalValidator and pydantic's decimal
validation reject the values below, which have 2 or fewer fractional digits
and too many integer digits.
"""

from decimal import Decimal

from models import Price, Quote


def record_big() -> Price:
    # BUG: six integer digits
    return Price.objects.create(amount=Decimal("123456.78"))


def record_negative() -> Price:
    # BUG: four integer digits below zero
    return Price.objects.create(amount=Decimal("-1000"))


def quote_big() -> Quote:
    # BUG: four integer digits into the pydantic field
    return Quote(amount=Decimal("1000.5"))
