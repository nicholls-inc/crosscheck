from django.db import models


class Customer(models.Model):
    first_name = models.TextField(null=True)
    last_name = models.TextField(null=True)
    phone = models.TextField(null=True)


class Contact(models.Model):
    first_name = models.TextField()
    last_name = models.TextField()
    phone = models.TextField()
