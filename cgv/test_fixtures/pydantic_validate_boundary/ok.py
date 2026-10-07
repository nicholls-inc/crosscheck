from decimal import Decimal

from models import Reading, four, maybe_label


def parse_flag(d: dict) -> Reading:
    # SAFE: lax validation coerces "true" to True
    return Reading.model_validate({"enabled": "true", "label": "a", "total": Decimal("1.00")})


def parse_label(d: dict) -> Reading:
    # SAFE: validation rejects None, by design
    return Reading.model_validate({"enabled": True, "label": maybe_label(d), "total": Decimal("1.00")})


def parse_total(x: Decimal) -> Reading:
    # SAFE: validation rejects a 4 decimal place total, by design
    return Reading.model_validate({"enabled": True, "label": "a", "total": four(x)})


def parse_json(raw: str) -> Reading:
    return Reading.model_validate_json(raw)


def parse_v1(d: dict) -> Reading:
    # SAFE: the pydantic v1 name for model_validate
    return Reading.parse_obj({"enabled": "true", "label": maybe_label(d), "total": four(Decimal(d["t"]))})
