from django.db import models


class Reading(models.Model):
    source = models.CharField(max_length=10)
    kwh = models.DecimalField(max_digits=10, decimal_places=3)
