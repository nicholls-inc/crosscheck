from typing import Any, Dict, Mapping, Optional

from records import Payer


def from_dict(data: dict) -> Payer:
    return Payer(name=data.get("name", ""), city="x")


def from_dict_any(data: Dict[str, Any]) -> Payer:
    return Payer(name=data.get("name", ""), city="x")


def from_optional_values(data: dict[str, Optional[str]]) -> Payer:
    return Payer(name=data.get("name", ""), city="x")


def from_union_values(data: Mapping[str, str | None]) -> Payer:
    return Payer(name=data.get("name", ""), city="x")


def from_optional_dict(data: Optional[dict]) -> Payer:
    if data is None:
        return Payer(name="", city="x")
    return Payer(name=data.get("name", ""), city="x")


def from_local(raw: str) -> Payer:
    fields = {"name": None if raw == "" else raw}
    return Payer(name=fields.get("name", ""), city="x")


def from_annotated_local(raw: str) -> Payer:
    fields: dict[str, str | None] = {"name": raw}
    return Payer(name=fields.get("name", ""), city="x")
