from typing import Optional

from fleet.models import Booking, Vehicle


def primary_vehicle(owner_id: int) -> Optional[Vehicle]:
    return Vehicle.objects.filter(owner_id=owner_id, is_primary=True).first()


def book_primary(owner_id: int) -> Booking:
    vehicle = primary_vehicle(owner_id)
    if vehicle is None:
        raise ValueError("no primary vehicle")
    return Booking.objects.create(plate=vehicle.plate)
