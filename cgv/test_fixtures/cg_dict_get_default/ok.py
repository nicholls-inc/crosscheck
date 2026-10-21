import os

from records import Payer

CITIES = {"LDN": "London", "MAN": "Manchester"}


def fixed(data: dict) -> Payer:
    return Payer(name=data.get("name") or "", city=data.get("city") or "")


def str_values(data: dict[str, str]) -> Payer:
    return Payer(name=data.get("name", ""), city="x")


def module_dict(code: str) -> Payer:
    return Payer(name="x", city=CITIES.get(code, "?"))


def untyped(payload) -> Payer:
    return Payer(name=payload.get("name", ""), city="x")


def environment() -> Payer:
    return Payer(name=os.environ.get("NAME", ""), city="x")


def key_in(data: dict) -> Payer:
    if "name" in data:
        return Payer(name=data.get("name", ""), city="x")
    return Payer(name="", city="x")
