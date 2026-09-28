from decimal import Decimal

from records import Lax, Reading, Row, StrictField, StrictModel


def as_float(p: float) -> float:
    return round(p, 2)


def as_decimal(p: Decimal) -> Decimal:
    return p.quantize(Decimal("0.01"))


def as_int(p: Decimal) -> int:
    return int(p)


def lax_ok(p: float, q: Decimal) -> Lax:
    # SAFE: pydantic lax mode coerces between int, float and Decimal
    return Lax(price=as_float(p), count=as_int(q), ratio=as_decimal(q))


def django_ok(p: float, q: Decimal) -> Reading:
    # SAFE: Django numeric fields convert
    return Reading.objects.create(kwh=as_decimal(q), cost=as_float(p), count=as_int(q))


def strict_model_bad(p: float) -> StrictModel:
    # BUG: strict mode rejects a float for a Decimal field
    return StrictModel(price=as_float(p))


def strict_field_bad(q: Decimal) -> StrictField:
    # BUG: strict int fields reject a Decimal
    return StrictField(count=as_decimal(q), other=as_int(q))


def dataclass_bad(p: float) -> Row:
    # BUG: annotation contract of a dataclass (no coercion)
    return Row(price=as_float(p))
