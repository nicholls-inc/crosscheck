from django.db import models


class Invoice(models.Model):
    number = models.CharField(max_length=12)
    currency = models.CharField(max_length=3)
    total = models.DecimalField(max_digits=10, decimal_places=2)
    points = models.PositiveIntegerField()


class AuditLog(models.Model):
    object_ref = models.CharField(max_length=12)
    short_ref = models.CharField(max_length=6)
    amount = models.DecimalField(max_digits=10, decimal_places=2)
    rate = models.DecimalField(max_digits=10, decimal_places=1)
    points = models.PositiveIntegerField()
