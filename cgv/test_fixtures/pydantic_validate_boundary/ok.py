from typing import Annotated, Optional

from pydantic import BaseModel, BeforeValidator, Field

from models import Reading


class Bounded(BaseModel):
    code: str = Field(max_length=3)
    note: Annotated[str, BeforeValidator(lambda v: v)]


def raw_label(d: dict) -> Optional[str]:
    return d.get("label")


def parse_flag(d: dict) -> Reading:
    # SAFE: lax validation coerces "true" to True
    return Reading.model_validate({"enabled": "true", "label": "a"})


def parse_label(d: dict) -> Reading:
    # SAFE: validation rejects None, by design
    return Reading.model_validate({"enabled": True, "label": raw_label(d)})


def parse_json(raw: str) -> Reading:
    return Reading.model_validate_json(raw)


def parse_v1(d: dict) -> Reading:
    # SAFE: the pydantic v1 name for model_validate
    return Reading.parse_obj({"enabled": "true", "label": raw_label(d)})


def parse_bounded(d: dict) -> Bounded:
    # SAFE: validation rejects a 4 character code and, after the BeforeValidator, a None note
    return Bounded.model_validate({"code": "abcd", "note": raw_label(d)})
