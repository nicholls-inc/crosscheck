from django.core.validators import MaxValueValidator
from django.db import models


class Account(models.Model):
    code = models.CharField(max_length=6)
    rate = models.DecimalField(max_digits=6, decimal_places=2)
    level = models.IntegerField(validators=[MaxValueValidator(10)])
    points = models.PositiveIntegerField()
    status = models.CharField(max_length=1, choices=[("a", "Active"), ("x", "Closed")])
    tier = models.IntegerField(choices=[(1, "Bronze"), (2, "Silver")])
    note = models.CharField(max_length=20)
    score = models.IntegerField(validators=[MaxValueValidator(5)])
