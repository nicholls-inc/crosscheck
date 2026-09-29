from dataclasses import dataclass

from django.db import models


class Depot(models.Model):
    name = models.CharField(max_length=40)


class Label(models.Model):
    text = models.CharField(max_length=20)


class Customer(models.Model):
    nickname = models.TextField(null=True)
    visits = models.IntegerField(default=0)


class Greeting(models.Model):
    nickname = models.TextField()


class Entry(models.Model):
    text = models.TextField()


class Marker(models.Model):
    code = models.TextField()
    level = models.IntegerField()


@dataclass
class Sample:
    value: float
