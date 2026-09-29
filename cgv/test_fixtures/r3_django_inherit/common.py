from django.db import models


class TimeStamped(models.Model):
    """Abstract base in another module: its fields belong to every child."""

    amount = models.DecimalField(max_digits=10, decimal_places=2)

    class Meta:
        abstract = True


class Place(models.Model):
    """Concrete parent (multi-table inheritance)."""

    name = models.CharField(max_length=5)
