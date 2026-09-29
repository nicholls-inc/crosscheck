from datetime import datetime

from billing.models import RenewalLog, Subscription


def record_renewal(renewed_at: datetime) -> RenewalLog:
    return RenewalLog.objects.create(renewed_at=renewed_at)


def renew(sub: Subscription) -> RenewalLog:
    return record_renewal(sub.renewed_at)  # type: ignore[arg-type]
