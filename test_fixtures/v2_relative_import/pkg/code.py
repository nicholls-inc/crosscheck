from decimal import Decimal

from .helpers import boost
from .records import Invoice


def make(a: Decimal) -> Invoice:
    return Invoice(total=boost(a))
