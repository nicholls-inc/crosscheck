from decimal import Decimal

from m import build


def caller_ok() -> None:
    # SAFE: 2dp into the 2dp field, through the forwarder
    build(fee=Decimal("1.25"))
