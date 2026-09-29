from django.db import models


class Basket(models.Model):
    OPEN, FROZEN = "Open", "Frozen"
    STATUS_CHOICES = ((OPEN, "Open"), (FROZEN, "Frozen"))
    status = models.CharField(max_length=6, choices=STATUS_CHOICES, default=OPEN)

    def freeze(self):
        self.status = self.FROZEN
        self.save()

    def bad(self):
        self.status = "Closed"
        self.save()
