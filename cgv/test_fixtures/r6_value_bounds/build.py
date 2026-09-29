"""Round 6 value facts: any slice `s[:n]` has length <= n; `max` / `min`
over a generator carry the elements' facts; `len(x) - k` is at least -k;
`Decimal(<float literal>)` has the exact places of the float; a literal
None into a field that accepts None meets its other requirements.
Final pass projects/f18, f20, adversarial probes q03.
"""

from decimal import Decimal

from records import Coupon, ExportSummary


def cents(v: Decimal) -> Decimal:
    return v.quantize(Decimal("0.01"))


def summarise(lines: list, token) -> ExportSummary:
    # SAFE: a 2-place sum and max, a slice of an unknown value
    return ExportSummary(
        total=sum(cents(l) for l in lines),
        largest=max(cents(l) for l in lines),
        line_count=max(1, len(lines)),
        code=token.upper()[:10],
    )


def summarise_bad(lines: list) -> ExportSummary:
    # BUG: len(lines) - 1 is 0 for one line
    return ExportSummary(
        total=sum(cents(l) for l in lines),
        largest=max(cents(l) for l in lines),
        line_count=len(lines) - 1,
        code="ok",
    )


def from_float() -> ExportSummary:
    # BUG: Decimal(0.1) has 55 decimal places
    return ExportSummary(total=Decimal(0.1), largest=Decimal(0.5), line_count=1, code="f")


def welcome() -> Coupon:
    # SAFE: None into a null=True positive field
    return Coupon.objects.create(percent_off=None, code="WELCOME")
