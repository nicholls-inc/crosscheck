from contextlib import suppress
from typing import Optional

import pytest


def lookup(table: dict, k: str) -> str:
    if k in table:
        return table[k]
    with suppress(ValueError):
        raise ValueError(k)


def checked(k: str) -> str:
    if k == "a":
        return "first"
    with pytest.raises(KeyError):
        raise KeyError(k)


def asserted(x: Optional[str]) -> str:
    with suppress(AssertionError):
        assert x is not None
    return x


def guarded(x: Optional[str]) -> str:
    with suppress(ValueError):
        if x is None:
            raise ValueError("missing")
    return x


def dereferenced(x: Optional[str]) -> str:
    with suppress(AttributeError):
        x.upper()
    return x
