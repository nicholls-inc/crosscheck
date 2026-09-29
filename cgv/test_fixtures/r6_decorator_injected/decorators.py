from functools import wraps


def background_task(func):
    """Supplies `db_session` when the caller leaves it out (dispatch's
    `@background_task`)."""

    @wraps(func)
    def wrapper(*args, **kwargs):
        if not kwargs.get("db_session"):
            kwargs["db_session"] = object()
        return func(*args, **kwargs)

    return wrapper
