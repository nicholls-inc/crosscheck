from dataclasses import dataclass


@dataclass
class Reading:
    amps: float
    impedance: complex


def ma_to_a(ma: float) -> float:
    return ma / 1000


def store(ma: float) -> Reading:
    return Reading(amps=ma_to_a(ma), impedance=0j)


def literal_into_float() -> Reading:
    return Reading(amps=3, impedance=0j)


def int_param_into_float(n: int) -> Reading:
    return Reading(amps=n, impedance=0j)


def int_and_float_into_complex(n: int, x: float) -> list[Reading]:
    return [Reading(amps=1.0, impedance=n), Reading(amps=1.0, impedance=x)]


def call_with_int(n: int) -> float:
    return ma_to_a(n)
