from django.core.validators import MaxValueValidator, MinValueValidator
from django.db import models


class Meter(models.Model):
    reading = models.IntegerField(validators=[MinValueValidator(0), MaxValueValidator(100)])
    count = models.PositiveIntegerField()
