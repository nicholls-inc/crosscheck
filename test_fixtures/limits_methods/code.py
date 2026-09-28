from decimal import Decimal

from records import Invoice


class Pricer:
    def rounded(self, a: Decimal) -> Decimal:
        return a.quantize(Decimal('0.0001'))

    def make(self, a: Decimal) -> Invoice:
        return Invoice(total=self.rounded(a), tax=Decimal('0'), customer="x")  # bug: 4dp > 2dp
