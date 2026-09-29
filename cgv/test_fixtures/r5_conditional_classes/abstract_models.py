from django.db import models


class AbstractPrice(models.Model):
    amount = models.DecimalField(max_digits=10, decimal_places=2)

    class Meta:
        abstract = True
