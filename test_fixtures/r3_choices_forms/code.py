from constants import BAD_COLOUR, DEFAULT_COLOUR
from models import Level, Order, Status, Ticket


def good_order() -> Order:
    # SAFE: every value is a declared choice
    return Order.objects.create(
        status=Status.ACTIVE, level=Level.HIGH, size="s", colour=DEFAULT_COLOUR, state="open"
    )


def bad_status() -> Order:
    # BUG: "pending" is not a Status value
    return Order.objects.create(status="pending")


def bad_level() -> Order:
    # BUG: 3 is not a Level value
    return Order.objects.create(level=3)


def bad_colour() -> Order:
    # BUG: module constant bound to a string that is not a choice
    return Order.objects.create(colour=BAD_COLOUR)


def any_size(size: str) -> Order:
    # Unknown value into a field with choices: warns
    return Order.objects.create(size=size)


def good_ticket() -> Ticket:
    # SAFE
    return Ticket(kind="bug")


def bad_ticket() -> Ticket:
    # BUG: not one of the Literal values
    return Ticket(kind="epic")
