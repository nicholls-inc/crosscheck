from django.db import models


class Price(models.Model):
    status = models.CharField(max_length=5, choices=[("a", "Active"), ("x", "Expired")])


def bad_choice() -> Price:
    return Price.objects.create(status="zz")
