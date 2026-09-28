from decimal import Decimal
from typing import Optional

from records import Invoice


def make(code: Optional[str]) -> Invoice:
    if code is None:
        raise ValueError("code required")
    return Invoice(total=Decimal('0'), tax=Decimal('0'), customer=code)  # narrowed: fine


def make_guarded(code: Optional[str]) -> Optional[Invoice]:
    if code is not None:
        return Invoice(total=Decimal('0'), tax=Decimal('0'), customer=code)  # fine
    return None


def make_unsafe(code: Optional[str]) -> Invoice:
    return Invoice(total=Decimal('0'), tax=Decimal('0'), customer=code)  # bug: may be None
