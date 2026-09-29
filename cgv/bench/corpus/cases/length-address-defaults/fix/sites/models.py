from django.db import models


class Registration(models.Model):
    full_address = models.CharField(max_length=255)
    owner = models.CharField(max_length=50)


class Site(models.Model):
    owner = models.CharField(max_length=50)
    address_line = models.CharField(max_length=100)
