from schemas import Battery, Hero, Settings


def full() -> Battery:
    # BUG: 101 > 100 (imported alias), 5000 > 4400 (local alias), 5 > 4 characters
    return Battery(level=101, voltage_mv=5000, code="ABCDE")


def load() -> Settings:
    # BUG: 0 < 1, 12 > 9 characters
    return Settings(workers=0, region="eu-west-9999")


def hero() -> Hero:
    # BUG: 9 > 8 characters
    return Hero(name="Superhero")
