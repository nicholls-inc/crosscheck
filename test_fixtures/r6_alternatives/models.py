from django.db import models


class StockItem(models.Model):
    qty = models.PositiveIntegerField()
    code = models.CharField(max_length=6)
    fee = models.DecimalField(max_digits=8, decimal_places=2)
