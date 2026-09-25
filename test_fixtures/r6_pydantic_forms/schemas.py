"""Round 6: `Annotated` aliases keep their constraints (also imported from
another module); `pydantic_settings.BaseSettings` and SQLModel classes are
pydantic classes. Final pass min/f19a, min/f06a.
"""

from typing import Annotated, Optional

from pydantic import BaseModel, Field
from pydantic_settings import BaseSettings
from sqlmodel import SQLModel

from types_ import Code, Percent

Voltage = Annotated[int, Field(ge=2500, le=4400)]


class Battery(BaseModel):
    level: Percent
    voltage_mv: Voltage
    code: Code


class Settings(BaseSettings):
    workers: int = Field(ge=1, le=16)
    region: str = Field(max_length=9)


class Hero(SQLModel, table=True):
    id: Optional[int] = Field(default=None, primary_key=True)
    name: str = Field(max_length=8)
