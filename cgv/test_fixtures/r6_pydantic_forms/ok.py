from schemas import Battery, Hero, Settings


def full_ok() -> Battery:
    return Battery(level=100, voltage_mv=4400, code="ABCD")


def load_ok() -> Settings:
    return Settings(workers=4, region="eu-west-1")


def hero_ok() -> Hero:
    return Hero(name="Hero")
