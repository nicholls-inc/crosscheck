from pydantic import BaseModel, Field
from typing import Literal


class Code(BaseModel):
    short: str = Field(max_length=5)
    kind: Literal["OK", "NO"]


def bad_length() -> Code:
    # BUG: "toolong".upper() is 7 characters
    return Code(short="toolong".upper(), kind="OK")


def bad_choice() -> Code:
    # BUG: "maybe".title() is "Maybe", not a choice
    return Code(short="x", kind="maybe".title())


def ok_mapped(flag: bool) -> Code:
    # SAFE: case mapping keeps the literal's length and maps its value
    return Code(short="abc".upper(), kind=("ok" if flag else "no").upper())


def ok_other() -> Code:
    return Code(short="ABCDE".lower(), kind="no".swapcase())
