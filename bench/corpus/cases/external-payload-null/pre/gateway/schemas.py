from pydantic import BaseModel


class QuoteResponse(BaseModel):
    currency: str
    price: str
