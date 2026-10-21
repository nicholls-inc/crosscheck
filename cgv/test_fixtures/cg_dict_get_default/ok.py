import os
from collections import Counter, defaultdict

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


def local_literal(raw: str) -> Payer:
    fields = {"name": raw, "city": "x"}
    return Payer(name=fields.get("name", ""), city="x")


def local_counter(words: list[str], w: str) -> int:
    counts = {}
    for x in words:
        counts[x] = counts.get(x, 0) + 1
    return counts.get(w, 0)


def local_dict_call(raw: str) -> Payer:
    fields = dict(name=raw)
    return Payer(name=fields.get("name", ""), city="x")


def local_comprehension(names: list[str]) -> Payer:
    fields = {n: n for n in names}
    return Payer(name=fields.get("name", ""), city="x")


def local_defaultdict() -> Payer:
    fields = defaultdict(str)
    return Payer(name=fields.get("name", ""), city="x")


def local_counter_class(words: list[str]) -> int:
    counts = Counter(words)
    return counts.get("x", 0)


def annotated_local(raw) -> Payer:
    fields: dict[str, str] = {"name": raw}
    return Payer(name=fields.get("name", ""), city="x")
