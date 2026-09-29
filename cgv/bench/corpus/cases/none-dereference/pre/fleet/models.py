from django.db import models


class Vehicle(models.Model):
    owner_id = models.IntegerField()
    name = models.TextField()
