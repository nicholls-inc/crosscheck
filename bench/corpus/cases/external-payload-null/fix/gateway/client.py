import requests

from gateway.schemas import QuoteResponse


def fetch_quote(url: str) -> QuoteResponse:
    payload = requests.get(url, timeout=10).json()
    return QuoteResponse(
        currency=payload.get("currency") or "GBP",
        price=payload.get("price") or "0",
    )
