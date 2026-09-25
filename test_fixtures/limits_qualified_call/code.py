from decimal import Decimal

from records import Invoice

import helpers

def make(first: str, last: str) -> Invoice:
    return Invoice(total=Decimal('0'), tax=Decimal('0'), customer=helpers.label(first, last))
