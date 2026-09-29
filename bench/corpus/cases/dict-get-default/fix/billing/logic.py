from billing.records import Payer


def parse_payer(data: dict) -> Payer:
    return Payer(
        name=data.get("name") or "",
        city=data.get("city") or "",
    )
