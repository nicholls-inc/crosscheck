from sites.models import Registration, Site


def site_for_registration(reg: Registration) -> Site:
    site, _created = Site.objects.get_or_create(
        owner=reg.owner,
        defaults={
            "address_line": reg.full_address[:100],
        },
    )
    return site
