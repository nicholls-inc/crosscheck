import requests

from gateway.schemas import QuoteResponse


def fetch_quote(url: str) -> QuoteResponse:
    response = requests.get(url, timeout=10)
    return QuoteResponse(**response.json())
