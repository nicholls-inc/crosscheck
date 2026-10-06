from django.db import models


class Customer(models.Model):
    nickname = models.TextField(null=True)


class Entry(models.Model):
    text = models.TextField()


def store(value: object) -> Entry:
    # BUG: `object` includes None, and the value reaches a non-null field
    return Entry.objects.create(text=value)


def store_nickname(customer: Customer) -> Entry:
    return store(customer.nickname)
