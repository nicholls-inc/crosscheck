from billing.models import Contact, Customer


def save_contact(first_name: str, last_name: str, phone: str) -> Contact:
    return Contact.objects.create(
        first_name=first_name,
        last_name=last_name,
        phone=phone,
    )


def sync_contact(customer: Customer) -> Contact:
    return save_contact(
        customer.first_name,  # type: ignore[arg-type]
        customer.last_name,  # type: ignore[arg-type]
        customer.phone,  # type: ignore[arg-type]
    )
