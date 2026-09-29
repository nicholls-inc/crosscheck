from records import Stock


def adjust(q: int) -> int:
    """ensures: result >= -50
    ensures: result <= 500
    """
    return q - 50

def make(q: int) -> Stock:
    return Stock(qty=adjust(q))
