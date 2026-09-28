"""Round 6: a read of a known class's field carries the field's declared
contracts (every write to the field is checked against them). Final pass
projects/f02, f08.
"""

from models import AuditLog, Invoice


def audit(instance: Invoice) -> AuditLog:
    # SAFE: number is at most 12 characters, total 2 places, points >= 0
    return AuditLog.objects.create(
        object_ref=instance.number,
        short_ref=instance.currency.upper()[:3],
        amount=instance.total,
        rate=round(instance.total, 1),
        points=instance.points + 1,
    )


def audit_short(instance: Invoice) -> AuditLog:
    # BUG: a 12-character number into a 6-character column
    return AuditLog.objects.create(
        object_ref="x",
        short_ref=instance.number,
        amount=instance.total,
        rate=round(instance.total, 1),
        points=0,
    )


def audit_rate(instance: Invoice) -> AuditLog:
    # BUG: 2 places into 1
    return AuditLog.objects.create(
        object_ref="x",
        short_ref="y",
        amount=instance.total,
        rate=instance.total,
        points=instance.points - 1,
    )
