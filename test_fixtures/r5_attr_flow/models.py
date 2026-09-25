from dataclasses import dataclass

from django.db import models


def lookup(d: dict):
    return d.get("k")


class Card(models.Model):
    card_type = models.CharField(max_length=20)
    label = models.CharField(max_length=5)

    def careless(self, d: dict) -> None:
        # BUG: saved while possibly None
        self.card_type = lookup(d)
        self.save()

    def one_branch(self, d: dict, strict: bool) -> None:
        # BUG: only one branch replaces None before the save
        self.card_type = lookup(d)
        if strict:
            if self.card_type is None:
                self.card_type = "unknown"
        self.save()

    def check_after_save(self, d: dict) -> None:
        # BUG: the None check comes after the save
        self.card_type = lookup(d)
        self.save()
        if self.card_type is None:
            self.card_type = "unknown"
            self.save()

    def long_label(self) -> None:
        # BUG: saved with a 7-character label
        self.label = "x"
        self.label = "toolong"
        self.save()


@dataclass
class Tag:
    name: str


def rename_bad(t: Tag, d: dict) -> None:
    # BUG: no save; the value reaching the function exit may be None
    t.name = d.get("n")
