from decimal import Decimal

from records import Entry, Job

Q4 = Decimal("0.0001")
Q2 = Decimal("0.01")
LONG = "x" * 30


def none_ok() -> Entry:
    # SAFE: None has no decimal places and no length
    return Entry(amount=Decimal("1.00"), limit=None, label="ok", note=None)


def repeated_bad() -> Entry:
    # BUG: "x" * 30 has length 30
    return Entry(amount=Decimal("1.00"), label="x" * 30)


def negative_bad() -> Entry:
    # BUG: -1.00 < 0
    return Entry(amount=Decimal("-1.00"), label="ok")


def sci(x: Decimal) -> Decimal:
    return x.quantize(Decimal("1e-4"))


def sci_bad(x: Decimal) -> Entry:
    # BUG: 1e-4 is 4 places
    return Entry(amount=sci(x), label="ok")


def const_bad(x: Decimal) -> Entry:
    # BUG: module constant Q4 is 4 places
    return Entry(amount=x.quantize(Q4), label="ok")


def const_ok(x: Decimal) -> Entry:
    # SAFE: module constants Q2 (2 places) and a short label
    return Entry(amount=abs(x).quantize(Q2), label="short")


def long_const_bad() -> Entry:
    # BUG: module constant of length 30
    return Entry(amount=Decimal("0"), label=LONG)


class Runner(Job):
    def label_entry(self) -> Entry:
        # SAFE: class constant "frozen" has length 6
        return Entry(amount=Decimal("0"), label=self.FROZEN)

    def rate_entry(self, x: Decimal) -> Entry:
        # BUG: class constant RATE has 3 places
        return Entry(amount=x.quantize(Job.RATE), label="r")
