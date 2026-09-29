from decimal import Decimal

from models import Price


def four(x: Decimal) -> Decimal:
    return x.quantize(Decimal("0.0001"))


def two(x: Decimal) -> Decimal:
    return x.quantize(Decimal("0.01"))


def goc_bad(x: Decimal) -> None:
    # BUG: `defaults` values are written on create and on update
    Price.objects.update_or_create(pk=1, defaults={"value": four(x)})


def lookup_bad(x: Decimal) -> None:
    # BUG: keyword lookups become field values when the row is created
    Price.objects.get_or_create(value=four(x))


def create_defaults_bad(x: Decimal) -> None:
    # BUG: Django 5 `create_defaults`
    Price.objects.update_or_create(pk=1, create_defaults={"value": four(x)}, defaults={"value": two(x)})


def goc_ok(x: Decimal) -> None:
    Price.objects.update_or_create(pk=1, defaults={"value": two(x)})
    Price.objects.get_or_create(value=two(x), defaults={"value": Decimal("1.00")})
