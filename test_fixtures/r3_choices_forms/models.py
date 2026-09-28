from typing import Literal

from django.db import models
from pydantic import BaseModel

from constants import COLOURS


class Status(models.TextChoices):
    ACTIVE = "active", "Active"
    EXPIRED = "expired", "Expired"


class Level(models.IntegerChoices):
    LOW = 1
    HIGH = 2


SIZES = [("s", "Small"), ("l", "Large")]


class Order(models.Model):
    FROZEN = "frozen"
    OPEN = "open"
    STATES = ((OPEN, "Open"), (FROZEN, "Frozen"))

    status = models.CharField(max_length=10, choices=Status.choices)
    level = models.IntegerField(choices=Level.choices)
    size = models.CharField(max_length=1, choices=SIZES)
    colour = models.CharField(max_length=5, choices=COLOURS)
    state = models.CharField(max_length=6, choices=STATES)

    def freeze(self):
        # SAFE: a class constant that is a valid choice
        self.state = self.FROZEN
        self.save()


class Ticket(BaseModel):
    kind: Literal["bug", "task"]
