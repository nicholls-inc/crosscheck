from django.db import models

from common import Place, TimeStamped


class Payment(TimeStamped):
    fee = models.DecimalField(max_digits=10, decimal_places=2)


class Restaurant(Place):
    seats = models.PositiveIntegerField()


class Precise(TimeStamped):
    # The child's definition replaces the inherited 2dp field.
    amount = models.DecimalField(max_digits=12, decimal_places=4)


class Grandchild(Payment):
    note = models.CharField(max_length=3)
