"""CG-1.28: guarded reads through a relation give no error."""

from models import Invoice, Order


def bill_guarded(order: Order) -> None:
    # NO ERROR: the guard narrows the chained email. Customer.name is non-null but
    # read through a link, which carries no declared guarantee: a warning.
    if order.customer.email is None:
        return
    Invoice.objects.create(email=order.customer.email, name=order.customer.name, short="x")


def bill_truthy(order: Order) -> None:
    # NO ERROR: a truthiness test narrows it too.
    if order.customer.email:
        Invoice.objects.create(email=order.customer.email, name="x", short="x")


def bill_referrer(order: Order) -> None:
    # NO ERROR: the nullable relation is guarded. Customer.name is non-null but read
    # through a link, which carries no declared guarantee: a warning.
    if order.referrer is not None:
        Invoice.objects.create(email="a", name=order.referrer.name, short="x")


def bill_reassigned(order: Order) -> None:
    # NO ERROR: the function stores a value into the chained field first.
    order.customer.email = "billing@example.com"
    Invoice.objects.create(email=order.customer.email, name="x", short="x")


def bill_referrer_unguarded(order: Order) -> None:
    # NOT AN ERROR: a None referrer raises AttributeError, so no None reaches
    # Invoice.name. The read is not known non-null either: a warning.
    Invoice.objects.create(email="a", name=order.referrer.name, short="x")


def bill_caller_guarded(order: Order) -> None:
    # NOT AN ERROR: the only caller narrows the chained field, but a caller the
    # project does not show may not, so the read is unknown: a warning (CG-1.9).
    Invoice.objects.create(email=order.customer.email, name="x", short="x")


def bill_when_known(order: Order) -> None:
    if order.customer.email:
        bill_caller_guarded(order)


def bill_after_rebind(order: Order, other: Order) -> None:
    # NOT AN ERROR: the guard narrowed the old customer; the link now points at
    # another one, so the narrowing is forgotten and the read is unknown: a warning.
    if order.customer.email is None:
        return
    order.customer = other.customer
    Invoice.objects.create(email=order.customer.email, name="x", short="x")


def bill_checked(order: Order) -> None:
    # NOT AN ERROR: an identity test on the link runs no code, so the only caller's
    # guard on the chained field still holds: a warning (CG-1.9).
    if order.customer is None:
        return
    Invoice.objects.create(email=order.customer.email, name="x", short="x")


def bill_checked_when_known(order: Order) -> None:
    if order.customer.email:
        bill_checked(order)


def bill_formatted(order: Order) -> None:
    # NOT AN ERROR: the nested function only reads the chain, so the only caller's
    # guard on the chained field still holds: a warning (CG-1.9).
    def label() -> str:
        return order.customer.name

    label()
    Invoice.objects.create(email=order.customer.email, name="x", short="x")


def bill_formatted_when_known(order: Order) -> None:
    if order.customer.email:
        bill_formatted(order)
