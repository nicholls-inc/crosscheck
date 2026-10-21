"""CG-1.28: a nullable field read through a nested data class gives an error where the target is non-null."""

from records import Label, Outer, Parcel, Payload


def label(payload: Payload) -> Label:
    # BUG: Address.postcode is Optional, Label.postcode is not.
    return Label(postcode=payload.address.postcode, city=payload.address.city)


def label_inner(outer: Outer) -> Label:
    # BUG: the same through a nested pydantic model.
    return Label(postcode=outer.inner.code, city="x")


class Courier(Parcel):
    def label_after_reset(self) -> Label:
        # BUG: the only caller narrows the chained field, but the parent's method
        # runs on this object first and clears it.
        super().reset()
        return Label(postcode=self.address.postcode, city="x")


def label_when_known(courier: Courier) -> Label:
    if courier.address.postcode:
        return courier.label_after_reset()
    return Label(postcode="x", city="x")
