from sites.models import Site, Ticket


def open_ticket(site: Site) -> Ticket:
    return Ticket.objects.create(
        site=site,
        operator_name=site.operator.display_name or "",
    )
