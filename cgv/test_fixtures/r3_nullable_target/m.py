from django.db import models


class Basket(models.Model):
    submitted_at = models.DateTimeField(null=True, blank=True)

    def submit(self, when):
        self.submitted_at = when
        self.save()
