from models import Customer, Depot, Greeting, Label


def label_depot(depot: Depot) -> Label:
    # TRUE BUG: 40-char name into a 20-char field
    return Label.objects.create(text=depot.name)


def greet(customer: Customer) -> Greeting:
    # TRUE BUG: a nullable nickname into a non-null field, reachable for any
    # customer without a nickname
    return Greeting.objects.create(nickname=customer.nickname)
