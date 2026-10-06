from typing import Any

from models import Customer, Entry


def remember(value: object) -> Entry:
    return Entry.objects.create(text=str(value))


def remember_any(value: Any) -> Entry:
    return Entry.objects.create(text=str(value))


def remember_bare(value) -> Entry:
    return Entry.objects.create(text=str(value))


def remember_nickname(customer: Customer) -> Entry:
    # SAFE: `object` accepts None and str() never returns None
    return remember(customer.nickname)


def remember_nickname_any(customer: Customer) -> Entry:
    # SAFE: `Any` accepts None
    return remember_any(customer.nickname)


def remember_nickname_bare(customer: Customer) -> Entry:
    # SAFE: an unannotated parameter accepts None
    return remember_bare(customer.nickname)
