"""CG-1.28: a field read through a nested data class carries the field's contracts."""

from records import Label, Outer, Payload


def label(payload: Payload) -> Label:
    # BUG: Address.postcode is Optional, Label.postcode is not.
    return Label(postcode=payload.address.postcode, city=payload.address.city)


def label_inner(outer: Outer) -> Label:
    # BUG: the same through a nested pydantic model.
    return Label(postcode=outer.inner.code, city="x")
