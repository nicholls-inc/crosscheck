from decimal import Decimal

import helpers as h

from records import Invoice


def make(first: str, last: str) -> Invoice:
    return Invoice(total=Decimal('0'), customer=h.label(first, last))
