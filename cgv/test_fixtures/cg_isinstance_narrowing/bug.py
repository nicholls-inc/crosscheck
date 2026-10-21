"""CG-1.14: an isinstance test that does not rule out None narrows nothing."""

from typing import Optional

from models import Address, Entry, Payload, clear


def store_negated(value: object) -> Entry:
    # BUG: the branch runs when value is not a str, None included.
    if not isinstance(value, str):
        return Entry.objects.create(text=value)
    return Entry.objects.create(text="x")


def store_none_type(value: object) -> Entry:
    # BUG: the tuple names NoneType, so None passes the test.
    if isinstance(value, (str, type(None))):
        return Entry.objects.create(text=value)
    return Entry.objects.create(text="x")


def store_object(value: object) -> Entry:
    # BUG: None is an instance of object.
    if isinstance(value, object):
        return Entry.objects.create(text=value)
    return Entry.objects.create(text="x")


def store_union_none(value: object) -> Entry:
    # BUG: `str | None` admits None.
    if isinstance(value, str | None):
        return Entry.objects.create(text=value)
    return Entry.objects.create(text="x")


def store_or(value: object, flag: bool) -> Entry:
    # BUG: the branch also runs when flag is true and value is None.
    if isinstance(value, str) or flag:
        return Entry.objects.create(text=value)
    return Entry.objects.create(text="x")


def store_param_shadow(value: object, str: type) -> Entry:
    # BUG: `str` is the parameter here, which may be NoneType.
    if isinstance(value, str):
        return Entry.objects.create(text=value)
    return Entry.objects.create(text="x")


def store_local_isinstance(value: object) -> Entry:
    # BUG: `isinstance` is a local function here, which may return True for None.
    isinstance = lambda v, t: True
    if isinstance(value, str):
        return Entry.objects.create(text=value)
    return Entry.objects.create(text="x")


def store_other_name(value: object, other: object) -> Entry:
    # BUG: the test is on another name.
    if isinstance(other, str):
        return Entry.objects.create(text=value)
    return Entry.objects.create(text="x")


def store_rebound(value: object, fallback: Optional[str]) -> Entry:
    # BUG: value is rebound to an Optional after the test.
    if isinstance(value, str):
        value = fallback
        return Entry.objects.create(text=value)
    return Entry.objects.create(text="x")


def store_chain_unguarded(payload: Payload) -> Entry:
    # BUG: the test is on the address, not on the postcode.
    if isinstance(payload.address, object):
        return Entry.objects.create(text=payload.address.postcode)
    return Entry.objects.create(text="x")


def store_method_of_result(value: object, other: Optional[str]) -> Entry:
    # BUG: the test is a method of the isinstance result, true for any other.
    if isinstance(value, str).__round__(other):
        return Entry.objects.create(text=other)
    return Entry.objects.create(text="x")


def store_chain_handed(payload: Payload) -> Entry:
    # BUG: clear() is handed the address after the test and sets the postcode to None.
    if isinstance(payload.address.postcode, str):
        clear(payload.address)
        return Entry.objects.create(text=payload.address.postcode)
    return Entry.objects.create(text="x")


def store_alias_before(payload: Payload) -> Entry:
    # BUG: other is the payload before the test, so code can write through it after.
    other = payload
    if isinstance(payload.address.postcode, str):
        other.address.postcode = None
        return Entry.objects.create(text=payload.address.postcode)
    return Entry.objects.create(text="x")


def store_link_alias_before(payload: Payload) -> Entry:
    # BUG: addr is the address before the test, so code can write through it after.
    addr = payload.address
    if isinstance(payload.address.postcode, str):
        addr.postcode = None
        return Entry.objects.create(text=payload.address.postcode)
    return Entry.objects.create(text="x")


def store_param_alias_before(address: Address) -> Entry:
    # BUG: other is the address before the test, so code can write through it after.
    other = address
    if isinstance(address.postcode, str):
        other.postcode = None
        return Entry.objects.create(text=address.postcode)
    return Entry.objects.create(text="x")


def store_closure_before(payload: Payload) -> Entry:
    # BUG: reset() clears the address, and it was made before the test.
    reset = lambda: clear(payload.address)
    if isinstance(payload.address.postcode, str):
        reset()
        return Entry.objects.create(text=payload.address.postcode)
    return Entry.objects.create(text="x")


def store_walrus_in_test(value: object, fallback: Optional[str]) -> Entry:
    # BUG: the walrus rebinds value to an Optional inside the test.
    if isinstance(value, str) and (value := fallback) is None:
        return Entry.objects.create(text=value)
    return Entry.objects.create(text="x")


def store_dict_write(address: Address) -> Entry:
    # BUG: the write goes through __dict__ and names no attribute.
    if isinstance(address.postcode, str):
        address.__dict__["postcode"] = None
        return Entry.objects.create(text=address.postcode)
    return Entry.objects.create(text="x")


def store_dict_alias_before(address: Address) -> Entry:
    # BUG: fields is the attribute table before the test, so code can write through it after.
    fields = address.__dict__
    if isinstance(address.postcode, str):
        fields["postcode"] = None
        return Entry.objects.create(text=address.postcode)
    return Entry.objects.create(text="x")


def store_nested_def_before(address: Address) -> Entry:
    # BUG: reset() writes the postcode, and it was defined before the test.
    def reset() -> None:
        address.postcode = None

    if isinstance(address.postcode, str):
        reset()
        return Entry.objects.create(text=address.postcode)
    return Entry.objects.create(text="x")


def store_nested_def_after(address: Address) -> Entry:
    # BUG: reset() writes the postcode, and it is defined inside the branch.
    if isinstance(address.postcode, str):
        def reset() -> None:
            address.postcode = None

        reset()
        return Entry.objects.create(text=address.postcode)
    return Entry.objects.create(text="x")


def store_nested_dict_before(address: Address) -> Entry:
    # BUG: reset() writes through __dict__, and it was defined before the test.
    def reset() -> None:
        address.__dict__["postcode"] = None

    if isinstance(address.postcode, str):
        reset()
        return Entry.objects.create(text=address.postcode)
    return Entry.objects.create(text="x")


def store_class_body_after(address: Address) -> Entry:
    # BUG: the class body runs at once and writes the postcode.
    if isinstance(address.postcode, str):
        class Clear:
            address.postcode = None

        return Entry.objects.create(text=address.postcode)
    return Entry.objects.create(text="x")


def store_walrus_in_later_operand(value: object, fallback: Optional[str]) -> object:
    # BUG: the walrus in a later operand rebinds value to an Optional.
    return isinstance(value, str) and ((value := fallback) or True) and Entry.objects.create(text=value)
