from billing.records import Payer


def parse_payer(data: dict) -> Payer:
    return Payer(
        name=data.get("name", ""),
        city=data.get("city", ""),
    )
