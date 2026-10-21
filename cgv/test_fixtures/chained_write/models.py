from django.db import models


class Customer(models.Model):
    name = models.CharField(max_length=40)


class Order(models.Model):
    customer = models.ForeignKey(Customer, on_delete=models.CASCADE)


class Invoice(models.Model):
    name = models.CharField(max_length=40)
