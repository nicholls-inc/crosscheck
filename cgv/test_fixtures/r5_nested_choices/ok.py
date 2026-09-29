from models import Invoice, Order, SpecialOrder


class InvoiceOps(Invoice):
    def send(self) -> None:
        # SAFE: a member of the nested class, through the model
        self.status = Invoice.Status.SENT
        self.save(update_fields=["status"])

    def pay(self) -> None:
        # SAFE: through self
        self.status = self.Status.PAID
        self.save()

    def reopen(self) -> None:
        self.status = "draft"
        self.save()


def good_kind() -> Order:
    return Order.objects.create(kind=Order.Kind.TWO)


def good_special() -> SpecialOrder:
    return SpecialOrder.objects.create(kind=SpecialOrder.Kind.ONE)
