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
