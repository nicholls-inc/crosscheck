from dataclasses import dataclass
from typing import Optional

from pydantic import BaseModel


@dataclass
class Address:
    city: str
    postcode: Optional[str]


@dataclass
class Payload:
    address: Address
    billing: Optional[Address] = None


class Inner(BaseModel):
    code: Optional[str] = None


class Outer(BaseModel):
    inner: Inner


class Label(BaseModel):
    postcode: str
    city: str
