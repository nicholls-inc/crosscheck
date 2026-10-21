"""CG-1.28: guarded reads through a nested data class stay clean."""

from typing import Optional

from records import Label, Payload


def label_guarded(payload: Payload) -> Optional[Label]:
    # SAFE: the guard narrows the chained name.
    if payload.address.postcode is None:
        return None
    return Label(postcode=payload.address.postcode, city=payload.address.city)


def label_truthy(payload: Payload) -> Optional[Label]:
    # SAFE: a truthiness test narrows it too.
    if payload.address.postcode:
        return Label(postcode=payload.address.postcode, city="x")
    return None


def label_billing(payload: Payload) -> Optional[Label]:
    # SAFE: the Optional nested class is guarded, and Address.city is a str.
    if payload.billing is not None:
        return Label(postcode="x", city=payload.billing.city)
    return None


def label_reassigned(payload: Payload) -> Label:
    # SAFE: the function stores a value into the chained field first.
    payload.address.postcode = "N1"
    return Label(postcode=payload.address.postcode, city="x")


def label_billing_unguarded(payload: Payload) -> Label:
    # NOT AN ERROR: a None billing raises AttributeError, so no None reaches
    # Label.city. The read is not known non-null either: a warning.
    return Label(postcode="x", city=payload.billing.city)
