"""CG-1.28 review: a None written through a chain is not checked yet."""

from models import Invoice, Order


def clear_name(order: Order) -> None:
    # Not checked yet: a write through a chain is no edge (docs/TASKS.md CG-1.52).
    order.customer.name = None


def copy_name(order: Order) -> None:
    # NOT CLEAN: Customer.name is declared non-null, but clear_name stores None
    # in it unchecked, so the chained read is not known non-null: a warning.
    Invoice.objects.create(name=order.customer.name)
