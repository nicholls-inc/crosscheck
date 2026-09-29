from django.db import models


class Price(models.Model):
    value = models.DecimalField(max_digits=10, decimal_places=2)
