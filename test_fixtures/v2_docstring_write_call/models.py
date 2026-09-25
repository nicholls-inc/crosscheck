from django.db import models


class Reading(models.Model):
    kwh = models.DecimalField(max_digits=8, decimal_places=3)
