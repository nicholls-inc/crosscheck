from fleet.models import Vehicle


def newest_vehicle_name(owner_id: int) -> str:
    vehicles = Vehicle.objects.filter(owner_id=owner_id).order_by("-id")
    return vehicles.first().name
