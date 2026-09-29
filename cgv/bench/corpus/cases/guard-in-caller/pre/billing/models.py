from django.db import models


class Subscription(models.Model):
    renewed_at = models.DateTimeField(null=True)


class RenewalLog(models.Model):
    renewed_at = models.DateTimeField()
