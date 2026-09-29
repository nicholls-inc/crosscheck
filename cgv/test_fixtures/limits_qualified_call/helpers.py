def label(first: str, last: str) -> str:
    """ensures: len(result) <= 64"""
    return (first + " " + last)[:64]
