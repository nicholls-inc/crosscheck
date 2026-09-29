"""Round 6: a `None`-default parameter of a function with a project-defined
decorator has unknown nullability inside it (the decorator may inject the
argument): a warning where it matters, never an error.
"""

from functools import lru_cache
from typing import Optional

from pydantic import BaseModel

from decorators import background_task


class Session:
    pass


class Audit(BaseModel):
    session_name: str


def record(db_session: Session, name: str) -> Audit:
    return Audit(session_name=name)


@background_task
def log_event(name: str, db_session=None):
    # SAFE (a warning): background_task injects db_session
    record(db_session, name)


@background_task
def log_event_typed(name: str, db_session: Optional[Session] = None):
    # SAFE (a warning): the same with an Optional annotation
    record(db_session, name)


def plain(name: str, db_session=None):
    # BUG: no decorator supplies db_session
    record(db_session, name)


@lru_cache
def cached(name: str, db_session=None):
    # BUG: lru_cache supplies nothing
    record(db_session, name)
