from dataclasses import dataclass
from decimal import Decimal

from django.db import models

from abstract_models import AbstractPrice


def is_model_registered(app, name):
    return False


# django-oscar's pattern: concrete models defined only when not overridden
# (adversarial2 min/k1).
if not is_model_registered("shop", "Price"):

    class Price(AbstractPrice):
        pass


try:

    class Fee(models.Model):
        amount = models.DecimalField(max_digits=10, decimal_places=2)

except ImportError:
    pass


if True:

    @dataclass
    class Point:
        x: int
        label: str

    def point_bad() -> "Point":
        # BUG: None into a non-Optional field (function defined in a module-level `if`)
        return Point(x=1, label=None)
