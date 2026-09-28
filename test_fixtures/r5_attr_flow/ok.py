from models import Card, Tag, lookup


class CardOps(Card):
    """Correct versions (round 5 N2: adversarial2 min/a1, oscar
    payment/abstract_models.py, netbox extras/models/customfields.py)."""

    def init_type(self, d: dict) -> None:
        # SAFE: a None lookup is replaced before the save
        self.card_type = lookup(d)
        if self.card_type is None:
            self.card_type = "unknown"
        self.save()

    def lock(self, d: dict) -> None:
        # SAFE: None raises before the save
        self.card_type = lookup(d)
        if self.card_type is None:
            raise ValueError("gone")
        self.save()

    def truthy(self, d: dict) -> None:
        # SAFE: saved only when set
        self.card_type = lookup(d)
        if self.card_type:
            self.save()

    def relabel(self) -> None:
        # SAFE: the long value is overwritten before the save
        self.label = "much too long"
        self.label = "short"
        self.save()

    def save(self, *args, **kwargs):
        # SAFE: normalised before the parent's save
        if self.card_type is None:
            self.card_type = "unknown"
        super().save(*args, **kwargs)


def local_version(d: dict) -> Card:
    t = lookup(d)
    if t is None:
        t = "unknown"
    return Card(card_type=t)


def rename(t: Tag, d: dict) -> None:
    # SAFE: None replaced before the function exits
    t.name = d.get("n")
    if t.name is None:
        t.name = "anon"


def rename_early(t: Tag, d: dict) -> None:
    # SAFE: returns before a None could stay
    t.name = d.get("n")
    if t.name is None:
        t.name = "anon"
        return
    t.name = t.name.strip()
