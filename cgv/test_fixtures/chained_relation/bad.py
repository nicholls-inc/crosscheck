"""CG-1.28: a nullable field read through a relation gives an error where the target is non-null."""

from models import Customer, Invoice, Ledger, Order, Shipment


def bill(order: Order) -> None:
    # BUG: Customer.email is null=True, Invoice.email is not.
    Invoice.objects.create(email=order.customer.email, name=order.customer.name, short="x")


def bill_shipment(shipment: Shipment) -> None:
    # BUG: the same read three levels deep, through a OneToOneField.
    Invoice.objects.create(email=shipment.order.customer.email, name="x", short="x")


def bill_short(order: Order) -> None:
    # NOT CLEAN: a 40-character name into a 10-character column. A write through
    # a chain is not checked yet (docs/TASKS.md CG-1.52), so the read carries no
    # declared length: a warning, not an error.
    Invoice.objects.create(email="a", name="b", short=order.customer.name)


def post_balance(order: Order) -> None:
    # NOT CLEAN: 4 decimal places into a 2-place column. As above, the chained
    # read carries no declared precision: a warning, not an error.
    Ledger.objects.create(balance=order.customer.balance)


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


def bill_deep_handed(shipment: Shipment) -> None:
    # BUG: the only caller narrows the chained field three levels deep, but this
    # function hands the customer, an object along the chain, to code that clears it.
    clear_email(shipment.order.customer)
    Invoice.objects.create(email=shipment.order.customer.email, name="x", short="x")


def bill_deep_when_known(shipment: Shipment) -> None:
    if shipment.order.customer.email:
        bill_deep_handed(shipment)


def bill_aliased(order: Order, customer: Customer) -> None:
    # BUG: the only caller passes the order's own customer as `customer`, so this
    # write clears the chained field the caller narrowed.
    customer.email = None
    Invoice.objects.create(email=order.customer.email, name="x", short="x")


def bill_aliased_when_known(order: Order) -> None:
    if order.customer.email:
        bill_aliased(order, order.customer)
