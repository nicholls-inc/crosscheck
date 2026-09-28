from decimal import Decimal

from pydantic import BaseModel, Field, field_validator, model_validator, validator


class Payment(BaseModel):
    """SAFE: the before-validator rounds to 2dp before decimal_places is checked
    (adversarial2 n23)."""

    amount: Decimal = Field(decimal_places=2)

    @field_validator("amount", mode="before")
    @classmethod
    def round_amount(cls, v):
        return Decimal(v).quantize(Decimal("0.01"))


class WrapPayment(BaseModel):
    amount: Decimal = Field(decimal_places=2)

    @field_validator("amount", mode="wrap")
    @classmethod
    def round_amount(cls, v, handler):
        return handler(Decimal(v).quantize(Decimal("0.01")))


class V1Payment(BaseModel):
    amount: Decimal = Field(decimal_places=2)

    @validator("amount", pre=True)
    def round_amount(cls, v):
        return Decimal(v).quantize(Decimal("0.01"))


class RootPayment(BaseModel):
    amount: Decimal = Field(decimal_places=2)

    @model_validator(mode="before")
    @classmethod
    def normalise(cls, data):
        data["amount"] = Decimal(data["amount"]).quantize(Decimal("0.01"))
        return data


class ChildPayment(Payment):
    """Inherits the base's before-validator."""


class AfterPayment(BaseModel):
    amount: Decimal = Field(decimal_places=2)

    @field_validator("amount")
    @classmethod
    def positive(cls, v):
        if v < 0:
            raise ValueError("negative")
        return v


class OtherFieldPayment(BaseModel):
    amount: Decimal = Field(decimal_places=2)
    note: str = ""

    @field_validator("note", mode="before")
    @classmethod
    def strip(cls, v):
        return str(v).strip()


class StrictPayment(BaseModel):
    amount: Decimal = Field(decimal_places=2)
