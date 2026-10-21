"""CG-1.14: module-level code is never narrowed by isinstance."""

from models import Entry

TEXT = None


def load():
    global TEXT
    TEXT = "a"


def reset():
    global TEXT
    TEXT = None


load()
# BUG: reset() sets TEXT to None between the test and the write, and the module's
# top-level code holds no def that shows it. origin/main reports this error.
if isinstance(TEXT, str):
    reset()
    Entry.objects.create(text=TEXT)
