from pydantic import BaseModel, Field


class Ticket(BaseModel):
    label: str = Field(max_length=16)
