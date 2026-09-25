from django.db import models


class TimeStamped(models.Model):
    class Meta:
        abstract = True


class Payment(TimeStamped):
    amount = models.DecimalField(max_digits=10, decimal_places=2)
