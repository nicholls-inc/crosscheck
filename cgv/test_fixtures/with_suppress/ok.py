import threading
from contextlib import suppress
from typing import Optional

lock = threading.Lock()


def returned(x: Optional[str]) -> str:
    with lock:
        if x is None:
            return "none"
        return x


def guarded_before(x: Optional[str]) -> str:
    if x is None:
        raise ValueError("missing")
    with suppress(KeyError):
        print(x)
    return x


def raised_after(k: str) -> str:
    if k == "a":
        return "first"
    with suppress(KeyError):
        print(k)
    raise ValueError(k)
