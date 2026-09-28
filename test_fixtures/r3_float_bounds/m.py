from decimal import Decimal
from pydantic import BaseModel, Field, condecimal


class R(BaseModel):
    ratio: float = Field(ge=0.0, le=0.5)
    amt: Decimal = Field(le=Decimal("10"))
    amt2: condecimal(ge=Decimal("0"))


def bad() -> R:
    return R(ratio=0.7, amt=Decimal("11"), amt2=Decimal("-1"))
