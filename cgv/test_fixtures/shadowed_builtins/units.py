class float:
    """A project type that shadows the builtin float."""

    def __init__(self, raw: str) -> None:
        self.raw = raw
