from pydantic import BaseModel


class Setting(BaseModel):
    value: str


def resolve(overrides: dict):
    value = overrides.get("k")
    if value is None:
        value = "unset"
    return value


def load(overrides: dict) -> Setting:
    return Setting(value=resolve(overrides))
