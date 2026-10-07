import builtins
from dataclasses import dataclass
from decimal import Decimal

from bad import Box, parsed
from records import Reading, measured


@dataclass
class Ledger:
    amount: Decimal
    count: builtins.int


def box_parsed() -> Box:
    # SAFE: a units.float into a units.float field
    return Box(value=parsed())


def store_measured() -> Reading:
    # SAFE: the builtin float and int
    return Reading(value=measured(), count=3)


def total() -> Decimal:
    return Decimal("1.00")


def ledger() -> Ledger:
    # SAFE: decimal.Decimal and builtins.int are the types they spell
    return Ledger(amount=total(), count=3)
