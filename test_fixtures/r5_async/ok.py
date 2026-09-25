from m import R, FxRate, fetch_4


async def store_ok() -> R:
    return R(rate=await fetch_4())


async def store_orm_ok() -> FxRate:
    return await FxRate.objects.acreate(pair="EURUSD", rate=await fetch_4())


async def reload(pk: int) -> None:
    fx = await FxRate.objects.aget(pk=pk)
    fx.rate = await fetch_4()
    await fx.asave()
