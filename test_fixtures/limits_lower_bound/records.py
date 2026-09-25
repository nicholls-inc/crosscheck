from pydantic import BaseModel, Field
class Stock(BaseModel):
    qty: int = Field(ge=0, le=1000)
