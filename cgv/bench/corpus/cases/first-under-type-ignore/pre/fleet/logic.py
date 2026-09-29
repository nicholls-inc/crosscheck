from fleet.models import Booking, Vehicle


def primary_vehicle(owner_id: int) -> Vehicle:
    return Vehicle.objects.filter(owner_id=owner_id, is_primary=True).first()  # type: ignore[return-value]


def book_primary(owner_id: int) -> Booking:
    vehicle = primary_vehicle(owner_id)
    return Booking.objects.create(plate=vehicle.plate)
