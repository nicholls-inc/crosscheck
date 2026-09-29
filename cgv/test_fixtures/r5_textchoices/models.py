from django.db import models


class Status(models.TextChoices):
    DRAFT = "draft", "Draft"
    PAID = "paid", "Paid"


class Invoice(models.Model):
    status = models.CharField(max_length=5, choices=Status.choices)

    def cancel(self) -> None:
        self.status = "cancl"
        self.save()

    def pay(self) -> None:
        self.status = Status.PAID
        self.save()
