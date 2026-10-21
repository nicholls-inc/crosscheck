"""CG-1.28 review: a chained read carries no declared length or precision."""

from records import Holder, Statement


def statement(holder: Holder) -> Statement:
    # NOT CLEAN: a write through a chain is not checked yet (docs/TASKS.md
    # CG-1.52), so Account.name and Account.balance read through the link have
    # no declared length or precision: a warning for each, not an error.
    return Statement(name=holder.account.name, balance=holder.account.balance)
