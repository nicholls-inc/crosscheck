from django.db import models


class Plan(models.Model):
    tier = models.IntegerField(choices=[(1, "Basic"), (2, "Pro")])
    code = models.CharField(max_length=5, choices=[("a,b", "AB"), ("c", "C")])


def ok() -> Plan:
    return Plan.objects.create(tier=1, code="a,b")


def bad() -> Plan:
    return Plan.objects.create(tier=3, code="a")
