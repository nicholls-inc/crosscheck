from datetime import datetime

from billing.models import RenewalLog, Subscription


def record_renewal(renewed_at: datetime) -> RenewalLog:
    return RenewalLog.objects.create(renewed_at=renewed_at)


def has_renewed(sub: Subscription) -> bool:
    return sub.renewed_at is not None


def renew(sub: Subscription) -> RenewalLog:
    if not has_renewed(sub):
        raise ValueError("subscription was never renewed")
    return record_renewal(sub.renewed_at)  # type: ignore[arg-type]
