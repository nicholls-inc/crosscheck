from django.db import models


class Operator(models.Model):
    display_name = models.TextField(null=True)


class Site(models.Model):
    operator = models.ForeignKey(Operator, on_delete=models.CASCADE)


class Ticket(models.Model):
    site = models.ForeignKey(Site, on_delete=models.CASCADE)
    operator_name = models.TextField()
