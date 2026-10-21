from django.conf import settings
from django.db import models


class Customer(models.Model):
    name = models.CharField(max_length=40)


class Profile(models.Model):
    customer = models.OneToOneField(Customer, on_delete=models.CASCADE, related_name="profile")
    email = models.CharField(max_length=254, null=True)


class Order(models.Model):
    customer = models.ForeignKey(Customer, on_delete=models.CASCADE)
    owner = models.ForeignKey(settings.AUTH_USER_MODEL, on_delete=models.CASCADE)
    agent = models.ForeignKey("auth.User", on_delete=models.CASCADE)


class Invoice(models.Model):
    email = models.CharField(max_length=254)
