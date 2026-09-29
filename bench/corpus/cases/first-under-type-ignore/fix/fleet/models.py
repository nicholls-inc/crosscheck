from django.db import models


class Vehicle(models.Model):
    owner_id = models.IntegerField()
    plate = models.TextField()
    is_primary = models.BooleanField(default=False)


class Booking(models.Model):
    plate = models.TextField()
