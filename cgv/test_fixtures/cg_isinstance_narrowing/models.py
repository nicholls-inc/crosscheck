from dataclasses import dataclass
from typing import Optional

from django.db import models


class Entry(models.Model):
    text = models.TextField()


class Text:
    pass


@dataclass
class Address:
    postcode: Optional[str]


@dataclass
class Payload:
    address: Address


def clear(address: Address) -> None:
    address.postcode = None
