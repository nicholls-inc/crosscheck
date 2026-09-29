from django.db import models


class EnergyRecord(models.Model):
    energy = models.DecimalField(max_digits=5, decimal_places=3)
    off_peak_energy = models.DecimalField(max_digits=5, decimal_places=3)
