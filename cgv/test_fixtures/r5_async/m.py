from decimal import Decimal

from django.db import models
from pydantic import BaseModel, Field


class R(BaseModel):
    rate: Decimal = Field(decimal_places=4)


class FxRate(models.Model):
    pair = models.CharField(max_length=6)
    rate = models.DecimalField(max_digits=10, decimal_places=4)


async def fetch() -> Decimal:
    return Decimal("1.234567")


async def fetch_4() -> Decimal:
    raw = await fetch()
    return raw.quantize(Decimal("0.0001"))


async def store() -> R:
    # BUG: 6 decimal places into 4 (adversarial2 min/m16)
    return R(rate=await fetch())


async def store_local() -> R:
    # BUG: the same through a local
    rate = await fetch()
    return R(rate=rate)


async def store_orm() -> FxRate:
    # BUG: an async ORM write (adversarial2 n16)
    return await FxRate.objects.acreate(pair="GBPUSD", rate=await fetch())


async def store_orm_update() -> None:
    # BUG: aupdate_or_create defaults
    await FxRate.objects.aupdate_or_create(pair="GBPEUR", defaults={"rate": await fetch()})
