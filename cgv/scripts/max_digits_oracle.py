#!/usr/bin/env python3
"""Check the extractor's max_digits range requirements against Django and pydantic.

For max_digits 1 to 6 and every decimal_places (or none the extractor can
read), the script writes a Django model and a pydantic model, runs the
extractor, reads each field's range requirement from the database, and
validates a grid of values with Django's DecimalValidator and with pydantic.

- Unsound: a value the library accepts lies outside the range. Always a failure.
- Inexact: with a readable decimal_places, a value inside the range with at
  most decimal_places fractional digits (as the library counts them) is
  rejected. A failure, except for zero when max_digits == decimal_places,
  which both libraries reject because they count its digit as a whole digit.

Usage (from cgv/, after building the extractor and the checker):
  uv run --no-project --with 'django==5.2.*' --with 'pydantic==2.*' \
    python scripts/max_digits_oracle.py
"""

import argparse
import sqlite3
import subprocess
import sys
import tempfile
from decimal import Decimal
from pathlib import Path

from django.core.exceptions import ValidationError as DjangoError
from django.core.validators import DecimalValidator
from pydantic import Field, ValidationError, create_model

MAX_DIGITS = range(1, 7)


def specs():
    for m in MAX_DIGITS:
        for d in [None, *range(0, m + 1)]:
            yield m, d


def name(m, d):
    return f"f_{m}_{'x' if d is None else d}"


def sources():
    django = ["from django.db import models", "", "", "class D(models.Model):"]
    pyd = ["from decimal import Decimal", "", "from pydantic import BaseModel, Field", "", "", "class P(BaseModel):"]
    for m, d in specs():
        places = "places" if d is None else d
        django.append(f"    {name(m, d)} = models.DecimalField(max_digits={m}, decimal_places={places})")
        kw = f"max_digits={m}" if d is None else f"max_digits={m}, decimal_places={d}"
        pyd.append(f"    {name(m, d)}: Decimal = Field({kw})")
    return "\n".join(django) + "\n", "\n".join(pyd) + "\n"


def ranges(cli, checker, root):
    db = root / "out.sqlite"
    subprocess.run(
        [cli, "contracts", "check", str(root / "app"), "--lean-checker", checker, "--output-db", str(db)],
        check=False,
        capture_output=True,
    )
    rows = sqlite3.connect(db).execute(
        "SELECT n.name, c.param_min_decimal, c.param_max_decimal FROM contracts c "
        "JOIN nodes n ON n.id = c.node_id "
        "WHERE c.constraint_type = 'range' AND c.contract_role = 'precondition' AND c.edge_id IS NULL"
    )
    return {node: (Decimal(lo), Decimal(hi)) for node, lo, hi in rows}


def values(m):
    yield Decimal("0")
    yield Decimal("0.00")
    yield Decimal("1E+2")
    yield Decimal("1.230")
    for whole in range(0, m + 3):
        for frac in range(0, m + 3):
            if whole == 0 and frac == 0:
                continue
            for digit in "19":
                w = digit * whole if whole else "0"
                f = "." + digit * frac if frac else ""
                yield Decimal(w + f)
                yield Decimal("-" + w + f)


def django_accepts(m, d, v):
    try:
        DecimalValidator(m, d)(v)
        return True
    except DjangoError:
        return False


def pydantic_model(m, d):
    kw = {"max_digits": m} if d is None else {"max_digits": m, "decimal_places": d}
    return create_model("M", x=(Decimal, Field(**kw)))


def pydantic_accepts(model, v):
    try:
        model(x=v)
        return True
    except ValidationError:
        return False


def places(v, normalise):
    return max(0, -(v.normalize() if normalise else v).as_tuple().exponent)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cli", default="target/release/crosscheck-contracts")
    parser.add_argument("--checker", default="prover/.lake/build/bin/contract-graph-checker")
    args = parser.parse_args()
    failures = checked = 0
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "app").mkdir()
        django_src, pyd_src = sources()
        (root / "app" / "dmodels.py").write_text(django_src)
        (root / "app" / "pmodels.py").write_text(pyd_src)
        found = ranges(args.cli, args.checker, root)
    for m, d in specs():
        model = pydantic_model(m, d)
        cases = [
            ("django", f"D.{name(m, d)}", False, [d] if d is not None else list(range(0, m + 1)),
             lambda v, dd: django_accepts(m, dd, v)),
            ("pydantic", f"P.{name(m, d)}", True, [d], lambda v, dd: pydantic_accepts(model, v)),
        ]
        for lib, node, normalise, real_places, accepts in cases:
            if node not in found:
                print(f"MISSING {lib} max_digits={m} decimal_places={d}: no range requirement")
                failures += 1
                continue
            lo, hi = found[node]
            for v in values(m):
                for dd in real_places:
                    checked += 1
                    ok = accepts(v, dd)
                    if ok and not lo <= v <= hi:
                        print(f"UNSOUND {lib} max_digits={m} decimal_places={d}: accepts {v}, range [{lo}, {hi}]")
                        failures += 1
                    zero_quirk = v == 0 and v.as_tuple().exponent == 0 and m == d
                    if (d is not None and not ok and lo <= v <= hi and places(v, normalise) <= d
                            and not zero_quirk):
                        print(f"INEXACT {lib} max_digits={m} decimal_places={d}: rejects {v}, range [{lo}, {hi}]")
                        failures += 1
    print(f"{checked} checks, {failures} failures")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
