from fleet.models import Trip, TripLabel


def label_trip(trip: Trip) -> TripLabel:
    return TripLabel.objects.create(
        text=trip.summary,
        distance_km=trip.distance_km,
    )
