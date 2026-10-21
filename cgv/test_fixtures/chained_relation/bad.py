"""CG-1.28: a field read through a relation carries the field's contracts."""

from models import Invoice, Order, Shipment


def bill(order: Order) -> None:
    # BUG: Customer.email is null=True, Invoice.email is not.
    Invoice.objects.create(email=order.customer.email, name=order.customer.name, short="x")


def bill_shipment(shipment: Shipment) -> None:
    # BUG: the same read three levels deep, through a OneToOneField.
    Invoice.objects.create(email=shipment.order.customer.email, name="x", short="x")


def bill_short(order: Order) -> None:
    # BUG: a 40-character name into a 10-character column.
    Invoice.objects.create(email="a", name="b", short=order.customer.name)


def clear_email(customer) -> None:
    customer.email = None


def bill_handed(order: Order) -> None:
    # BUG: the only caller narrows the chained field, but this function hands the
    # customer to code that clears it first, so the caller's guard no longer holds.
    clear_email(order.customer)
    Invoice.objects.create(email=order.customer.email, name="x", short="x")


def bill_when_known(order: Order) -> None:
    if order.customer.email:
        bill_handed(order)


def bill_nested(order: Order) -> None:
    # BUG: the nested function clears the chained field when it is called, so the
    # caller's guard no longer holds.
    def clear() -> None:
        order.customer.email = None

    clear()
    Invoice.objects.create(email=order.customer.email, name="x", short="x")


def bill_nested_when_known(order: Order) -> None:
    if order.customer.email:
        bill_nested(order)
