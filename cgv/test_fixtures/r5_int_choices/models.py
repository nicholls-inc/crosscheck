from django.db import models


class Level(models.IntegerChoices):
    LOW = 1, "Low"
    HIGH = 2, "High"


class Plan(models.Model):
    tier = models.IntegerField(choices=Level.choices)
    code = models.CharField(max_length=5, choices=[("a", "A"), ("b", "B")])


def set_code(p: Plan, code: str) -> None:
    p.code = code
    p.save()


def set_tier_ok() -> Plan:
    return Plan.objects.create(tier=Level.HIGH, code="a")


def set_tier_bad() -> Plan:
    return Plan.objects.create(tier=5, code="b")


def set_code_fmt(n: int) -> Plan:
    return Plan.objects.create(tier=1, code=f"a{n}")
