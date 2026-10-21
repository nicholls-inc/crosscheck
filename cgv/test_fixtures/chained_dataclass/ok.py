"""CG-1.28: guarded reads through a nested data class give no error."""

from typing import Optional

from records import Label, Payload


def label_guarded(payload: Payload) -> Optional[Label]:
    # NO ERROR: the guard narrows the chained postcode. Address.city is a str but read
    # through a link, which carries no declared guarantee: a warning.
    if payload.address.postcode is None:
        return None
    return Label(postcode=payload.address.postcode, city=payload.address.city)


def label_truthy(payload: Payload) -> Optional[Label]:
    # NO ERROR: a truthiness test narrows it too.
    if payload.address.postcode:
        return Label(postcode=payload.address.postcode, city="x")
    return None


def label_billing(payload: Payload) -> Optional[Label]:
    # NO ERROR: the Optional nested class is guarded. Address.city is a str but read
    # through a link, which carries no declared guarantee: a warning.
    if payload.billing is not None:
        return Label(postcode="x", city=payload.billing.city)
    return None


def label_reassigned(payload: Payload) -> Label:
    # NO ERROR: the function stores a value into the chained field first.
    payload.address.postcode = "N1"
    return Label(postcode=payload.address.postcode, city="x")


def label_billing_unguarded(payload: Payload) -> Label:
    # NOT AN ERROR: a None billing raises AttributeError, so no None reaches
    # Label.city. The read is not known non-null either: a warning.
    return Label(postcode="x", city=payload.billing.city)
