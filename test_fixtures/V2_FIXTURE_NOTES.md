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
