from dataclasses import dataclass
from decimal import Decimal
from typing import Optional

from pydantic import BaseModel, Field


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


class Account(BaseModel):
    name: str = Field(max_length=40)
    balance: Decimal = Field(max_digits=12, decimal_places=4)


@dataclass
class Holder:
    account: Account


class Statement(BaseModel):
    name: str = Field(max_length=10)
    balance: Decimal = Field(max_digits=12, decimal_places=2)
