from django.db import models


class Trip(models.Model):
    summary = models.CharField(max_length=200)
    distance_km = models.IntegerField()


class TripLabel(models.Model):
    text = models.CharField(max_length=17)
    distance_km = models.IntegerField()
