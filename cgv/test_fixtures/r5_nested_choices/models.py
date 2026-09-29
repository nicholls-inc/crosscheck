from django.db import models


class Invoice(models.Model):
    # A choices class nested in the model (adversarial2 min/m09b, n09).
    class Status(models.TextChoices):
        DRAFT = "draft", "Draft"
        SENT = "sent", "Sent"
        PAID = "paid", "Paid"
        VOID = "void", "Void"

    number = models.CharField(max_length=10)
    status = models.CharField(max_length=5, choices=Status.choices, default=Status.DRAFT)

    def cancel(self) -> None:
        # BUG: "cancl" fits max_length but is not a choice
        self.status = "cancl"
        self.save()

    def refund(self) -> None:
        # BUG: "refunded" is neither a choice nor at most 5 characters
        self.status = "refunded"
        self.save()


class Order(models.Model):
    class Kind(models.IntegerChoices):
        ONE = 1, "One"
        TWO = 2, "Two"

    kind = models.IntegerField(choices=Kind.choices)


class SpecialOrder(Order):
    pass


def cancel_all() -> None:
    # BUG: the same through a queryset update
    Invoice.objects.update(status="cancl")


def bad_kind() -> Order:
    # BUG: 3 is not a Kind
    return Order.objects.create(kind=3)
