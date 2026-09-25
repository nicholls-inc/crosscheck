# v2 fixture notes

Reasoning for cases where `docs/design/dataflow-v2.md` did not settle the
expected verdict unambiguously. Each fixture's `expected.json` was derived
from the most consistent reading of the spec's own stated rules; these are
the spots where more than one reading was plausible.

## `v2_namedtuple_attrs_ctor` — nullability of a non-`None` literal override

`make_point` builds `Point("fixed", v)`: the first positional argument is a
plain string literal, not a call to an extracted function, so per the model
table it becomes an **override** whose postconditions are "the contracts of
`expr` in F". The spec explicitly enumerates what makes a value nullable
(the literal `None`, `a if c else None`, `.get()`, `getattr(..., None)`,
etc.) and separately calls out that `Cls(field=None)` needed to become
visible as a v1 fix — but it never states what a *non-`None`* literal's
nullability contract is. Two readings are possible: (a) a bare non-`None`
literal is trivially known to be non-null, so the override carries an
explicit `non-null` postcondition and the edge is silently consistent; or
(b) nullability is only ever asserted positively (mirroring how unknown
*precision* is left unset rather than defaulted, e.g. `limits_arith_unknown`),
so a plain literal carries no nullability postcondition at all and the edge
falls under "Unknown values warn". This fixture assumes reading (b) — no
error, but a required warning containing `"nullability"` on
`make_point -> Point.label` — because it is the reading consistent with the
spec's general default-to-unknown posture elsewhere (precision, length,
range all default to "no postcondition" rather than an assumed-safe bound),
and because a silently-assumed non-null bound for arbitrary literals is the
riskier (unsound-feeling) direction to bake into a fixture's expectation.

## `v2_dependent_resolved` / `v2_dependent_unknown_multiparam` — the worked example's parameter count

The spec's own illustration of a body-derived dependent bound is
`x = p.quantize(Decimal('0.001')) if c else p`, immediately followed by "so
it is only emitted when F has exactly one parameter (ignoring `self`/`cls`)".
Taken literally, a function shaped like that example — with both `p` and `c`
in scope — would already have two parameters and so, by the very next
sentence, should *not* get a dependent bound. The two statements read as
mutually inconsistent if `c` is meant to be a second formal parameter.
Rather than guess which of the two statements the implementation favors,
both dependent fixtures avoid the ambiguity entirely: `clamp_floor`'s branch
condition in `v2_dependent_resolved` is derived from the sole parameter `p`
itself (`p.as_tuple().exponent < floor.as_tuple().exponent`, `floor` being a
local constant, not a parameter), so the "exactly one parameter" rule
applies unambiguously and the dependent bound is expected to resolve through
composition. `v2_dependent_unknown_multiparam` gives `clamp_floor` a second
*formal* parameter (`floor: Decimal`) so the "more parameters" branch applies
unambiguously instead. Neither fixture depends on resolving what `c` in the
spec's own example is supposed to be.

## `v2_dependent_unknown_multiparam` — which hop carries the vacuous-pass warning

With `clamp_floor` unable to produce a precision postcondition, a path
starting at `make4dp` (`make4dp -> clamp_floor -> Invoice.total`) and a path
starting at `clamp_floor` alone (`clamp_floor -> Invoice.total`) both reach
the same missing-precision finding. `Main.lean`'s `runChecker` dedupes
findings that differ only in path, keeping the shortest, so the fixture
expects the warning on the single-hop path. This isn't stated for warnings
explicitly (the existing dedup note is phrased around findings generally),
but it is the reading consistent with `limits_per_argument`, whose analogous
vacuous-precision warning is likewise attributed to the short hop
(`combine -> Invoice.total`) rather than the longer composed path.

## `v2_none_producers` — matching `qs.first()` / `qs.last()` syntactically

`pick_latest` returns
`Sample.objects.order_by("-id").values_list("amount", flat=True).first()`,
a realistic Django idiom for reading a possibly-absent scalar. The spec's
rule is phrased as the syntactic pattern "`qs.first()` / `qs.last()`" without
requiring a specific receiver, matching how `PRICES.get(sku)` on a plain
`dict` (not a Django queryset) is already treated as a None producer in
`limits_dict_get`. This fixture assumes the same name-based matching applies
to any call chain ending in `.first()`/`.last()`, regardless of what precedes
it.

## Resolutions (orchestrator)

- Non-`None` literals are non-null: reading (a). A literal's value is known,
  so recording nullability 0 is a fact rather than a guess, and the spec now
  says so. `v2_namedtuple_attrs_ctor` no longer requires a warning on
  `make_point -> Point.label`.
- The spec's dependent-bound example now uses a condition on the single
  parameter itself, so it no longer suggests a second parameter.
- Deduplication applies to warnings too; `.first()`/`.last()` match on any
  receiver. Both as assumed here.

## Round 3 fixtures (`r3_*`)

One fixture per adversarial repro (`r3_crash` .. `r3_nullable_target`,
`docs/design/dataflow-v2.md` "Round 3") and per D6/D7 probe
(`r3_module_level_django`, `r3_update_or_create`, `r3_create_kwargs_fwd`),
plus `r3_django_inherit`, `r3_choices_forms`, `r3_literals` and
`r3_numeric_types` for the extraction rules the repros only touch. Where a
repro is a single bug, the fixture may add an `ok.py` with the corrected
form, so an error on correct code fails the fixture.

- Findings at different write sites are not merged: `r3_dedup` expects the
  same error three times (one per site), `r3_update_or_create` likewise.
- `r3_context` and `r3_noise` expect no errors; unresolved-dependent
  warnings may still appear on paths that start at a call-site node.
- `r3_choices_forms` requires the "no choices fact" warning for a write of
  an unknown string into a field with choices.
- Range bounds display as decimals (`range ≤ 0.7`); choices as
  `choices in [a, x]`.
