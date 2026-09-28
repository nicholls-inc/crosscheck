from decimal import Decimal

from schemas import (
    AfterPayment,
    ChildPayment,
    OtherFieldPayment,
    Payment,
    RootPayment,
    StrictPayment,
    V1Payment,
    WrapPayment,
)


def four(x: Decimal) -> Decimal:
    return x.quantize(Decimal("0.0001"))


def pay(x: Decimal) -> Payment:
    return Payment(amount=four(x))


def pay_wrap(x: Decimal) -> WrapPayment:
    return WrapPayment(amount=four(x))


def pay_v1(x: Decimal) -> V1Payment:
    return V1Payment(amount=four(x))


def pay_root(x: Decimal) -> RootPayment:
    return RootPayment(amount=four(x))


def pay_child(x: Decimal) -> ChildPayment:
    return ChildPayment(amount=four(x))


def pay_after(x: Decimal) -> AfterPayment:
    # BUG: an after-validator sees the 4dp value only once it has been validated
    return AfterPayment(amount=four(x))


def pay_other(x: Decimal) -> OtherFieldPayment:
    # BUG: the before-validator is on another field
    return OtherFieldPayment(amount=four(x))


def pay_strict(x: Decimal) -> StrictPayment:
    # BUG: no rounding validator
    return StrictPayment(amount=four(x))
