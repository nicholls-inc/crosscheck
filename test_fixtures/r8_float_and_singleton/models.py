"""Review fixes (PR #3, pr-swarm round 1): guarantees that were tighter than
the value written.

- A FloatField read is a binary float: `r.a + r.b` with a <= 0.1, b <= 0.2 is
  0.30000000000000004 when both are at their maximum, above 0.3, so the sum
  has no exact upper bound.
- `max([x])` returns its one element without comparing it, so it is None
  when `x` is.
"""

from django.core.validators import MaxValueValidator
from django.db import models


class Reading(models.Model):
    a = models.FloatField(validators=[MaxValueValidator(0.1)])
    b = models.FloatField(validators=[MaxValueValidator(0.2)])


class Total(models.Model):
    total = models.FloatField(validators=[MaxValueValidator(0.3)])
    n = models.IntegerField()


def combine(r: Reading) -> Total:
    return Total.objects.create(total=r.a + r.b, n=1)


def peak(x):
    vals = [x]
    return Total.objects.create(total=0.0, n=max(vals))


def peak_literal(x):
    return Total.objects.create(total=0.0, n=max([x]))


def larger(x, y):
    return Total.objects.create(total=0.0, n=max([x, y]))
