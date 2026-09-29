from pydantic import BaseModel, Field

# Bounds beyond 9.22e12: their micros overflow a 64-bit integer, so only the
# exact decimal columns (round 5) carry them (adversarial2 min/e1).
# Intended verdict; passes once the checker reads param_*_decimal.


class Big(BaseModel):
    n: int = Field(le=10_000_000_000_000)
    m: int = Field(ge=-10_000_000_000_000)


def ok() -> Big:
    return Big(n=5, m=5)


def edge() -> Big:
    return Big(n=10_000_000_000_000, m=-10_000_000_000_000)


def bad() -> Big:
    # BUG: both bounds exceeded
    return Big(n=20_000_000_000_000, m=-20_000_000_000_000)
