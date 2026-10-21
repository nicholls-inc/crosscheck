from django.db import models


class Customer(models.Model):
    email = models.CharField(max_length=254, null=True)
    name = models.CharField(max_length=40)
    balance = models.DecimalField(max_digits=12, decimal_places=4)


class Order(models.Model):
    customer = models.ForeignKey(Customer, on_delete=models.CASCADE)
    referrer = models.ForeignKey(
        "Customer", null=True, on_delete=models.SET_NULL, related_name="+"
    )


class Shipment(models.Model):
    order = models.OneToOneField(Order, on_delete=models.CASCADE)


class Invoice(models.Model):
    email = models.CharField(max_length=254)
    name = models.CharField(max_length=40)
    short = models.CharField(max_length=10)


class Ledger(models.Model):
    balance = models.DecimalField(max_digits=12, decimal_places=2)
