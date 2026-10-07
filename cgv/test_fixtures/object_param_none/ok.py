from typing import Any, Union

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


def remember_union(value: Union[object, int]) -> Entry:
    return Entry.objects.create(text=str(value))


def remember_bitor(value: object | int) -> Entry:
    return Entry.objects.create(text=str(value))


def remember_nickname_union(customer: Customer) -> Entry:
    # SAFE: `Union[object, int]` is `object`, which accepts None
    return remember_union(customer.nickname)


def remember_nickname_bitor(customer: Customer) -> Entry:
    # SAFE: `object | int` is `object`, which accepts None
    return remember_bitor(customer.nickname)
