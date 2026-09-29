from models import Marker


def find_code(raw: str) -> str:
    # BENIGN: the annotation is wrong, but every caller checks for None
    if not raw:
        return None
    return raw.strip().upper()


def mark_one(raw: str) -> None:
    code = find_code(raw)
    if code is None:
        return
    Marker.objects.create(code=code, level=1)


def mark_many(raws: list) -> int:
    count = 0
    for raw in raws:
        code = find_code(raw)
        if code is not None:
            Marker.objects.create(code=code, level=2)
            count += 1
    return count
