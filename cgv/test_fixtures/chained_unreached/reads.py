"""CG-1.28: a chain CGV does not reach reads as unknown. That is a warning, never silence and never an error."""

from typing import Optional

from models import Customer, Invoice, Order


def get_order() -> Optional[Order]:
    return Order.objects.first()


def bill_reverse(customer: Customer) -> None:
    # NOT YET REACHED: `profile` is the reverse side of a OneToOneField. A link is
    # resolved only from the field's own declaration, so the read is unknown.
    Invoice.objects.create(email=customer.profile.email)


def bill_auth_user_model(order: Order) -> None:
    # NOT YET REACHED: the target is named by settings.AUTH_USER_MODEL.
    Invoice.objects.create(email=order.owner.email)


def bill_unresolvable(order: Order) -> None:
    # NOT YET REACHED: "auth.User" names no project model.
    Invoice.objects.create(email=order.agent.email)


def bill_call_root() -> None:
    # NOT YET REACHED: the chain's root is a call result, not a local of a known class.
    Invoice.objects.create(email=get_order().customer.profile.email)
