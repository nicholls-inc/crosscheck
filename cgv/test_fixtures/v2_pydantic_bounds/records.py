from pydantic import BaseModel, Field


class Item(BaseModel):
    qty: int = Field(gt=0)
    pct: int = Field(ge=0, le=100)
