# Data-flow model v2

Status: adopted 2026-09-25. Replaces the "one postcondition set per function"
model for writes and call arguments.

## Problems this fixes

Each item has a fixture under `test_fixtures/limits_*` whose `expected.json`
states the intended verdict.

| # | Problem in v1 | Fixture |
|---|---------------|---------|
| 1 | `obj.field = v` (and Django `obj.save()`) is not a write | `limits_attr_assign`, `limits_django_save` |
| 2 | `Cls(**data)` is not a write | `limits_kwargs` |
| 3 | `module.func(...)` is not followed | `limits_qualified_call` |
| 4 | `Cls(field=None)` is not seen | `limits_literal_none` |
| 5 | `d.get(k)` and similar None producers give no nullability | `limits_dict_get` |
| 6 | a write whose precision is unknown passes silently | `limits_arith_unknown` |
| 7 | lower bounds (`ge`, `>=`, `MinValueValidator`) are not checked | `limits_lower_bound` |
| 8 | two classes with one name in different modules collapse into one | `limits_name_clash` |
| 9 | one precision guarantee per function is applied to every field it writes | `limits_per_field` |
| 10 | a function's preconditions are applied to every argument | `limits_per_argument` |
| 11 | a finding is attributed to the path's last node, not the failing hop | `limits_per_argument` |
| 12 | unknown arguments are skipped when a function-wide bound is computed | `limits_per_argument` |

## Model

Nodes stay `function` and `model` (a Django or data class field).

- A **function node's postconditions describe its return value only**: return
  annotation, body analysis of `return` expressions, docstring `ensures:`.
- A **function node's preconditions describe its parameters**. Every
  precondition row has a `subject`: the parameter name. Sources: parameter
  annotations (type; nullability 0 unless `Optional[T]` / `T | None`) and
  docstring `requires:` clauses (`precision(amount) <= 2` has subject
  `amount`). A row with `subject` NULL applies to every parameter (legacy).

Edges:

| Code | Edge | Source side | Target side |
|------|------|-------------|-------------|
| `Cls(f=g(...))`, `x = g(...); Cls(f=x)`, same for `objects.create`, `**{...}`, positional, `obj.f = g(...)` | `g writes_to Cls.f` | g's return postconditions | field preconditions |
| `Cls(f=expr)` etc. where `expr` is not a call to an extracted function | `F writes_to Cls.f` (F = enclosing function) | **override**: contracts of `expr` in F | field preconditions |
| `h(..., g(...), ...)`, or via a local bound to `g(...)` | `g flows_to h`, `target_param` = the parameter it binds | g's return postconditions | h's preconditions whose subject is that parameter (or NULL) |
| `h(..., expr, ...)` with `expr` not such a call | `F flows_to h`, `target_param` set | **override**: contracts of `expr` in F | as above |
| `h(...)` | `F calls h` | structural only, **not checked** | |

An **override** means: for this edge, the source node's postconditions are
replaced by the edge's own contract rows (possibly none). This is how one
function can write a 2dp value to one field and a 4dp value to another.

### Values that depend on the enclosing function's input

A value derived from a parameter of F gets a *dependent* bound, written in the
existing DepExpr grammar with `input_<kind>` (e.g. `max(3, input_precision)`
for `x = p.quantize(Decimal('0.001')) if c else p`). `input_<kind>` binds to
whatever flows into F along the path being checked, so it is only emitted when
F has exactly one parameter (ignoring `self`/`cls`). With more parameters, a
parameter-derived precision/length/range is unknown. Nullability and type of a
parameter-derived value come from the parameter's annotation (static).

A path that starts at F leaves such a bound unresolved and produces the
existing "could not be resolved" warning. A path that reaches F through a
`flows_to` edge resolves it. This keeps the transitive fixture's point:
checking the `split_energy -> EnergyRecord.energy` edge alone cannot decide it,
the composed path can.

### Docstring `ensures:` on write edges

When F's return expression is itself the write call (`return Cls(...)` or
`return Model.objects.create(...)`), F's docstring `ensures:` postconditions
(ASSUMED) are added to that call's override rows for any kind the extraction
did not determine. Otherwise docstrings describe only the return value.

### Nullability narrowing

Within F, a name is treated as non-None after `if x is None: <return|raise>`,
inside `if x is not None:` / `if x:` bodies, and after `assert x is not None`.
Flow-insensitive otherwise.

### None producers

A value is nullable (1) when it is: the literal `None`; `a if c else None` (either
branch None); `m.get(k)` with one argument or default `None`; `getattr(o, n, None)`;
`next(it, None)`; `d.pop(k, None)`; `re.match/search/fullmatch(...)`;
`qs.first()` / `qs.last()`; a call to an extracted function whose return is
nullable (fixpoint across the project); a parameter annotated `Optional`.
A function without a return annotation whose returns include any of these gets
a nullability-1 postcondition (EXTRACTED).

### Lower bounds

Range rows carry `param_min_value` and/or `param_max_value`. The checker reads
a row as up to two constraints: `range` (upper, `≤`) and `rangeMin` (lower,
`≥`, consistent iff source min ≥ target min). Sources of lower bounds: pydantic
`ge=n`, `gt=n` on int (`n+1`), docstring `NAME >= n`, Django
`MinValueValidator(n)`, `PositiveIntegerField` / `PositiveSmallIntegerField` /
`PositiveBigIntegerField` (0). Upper: `le`, `lt` on int, `MaxValueValidator(n)`.
Display: `range ≥ n`.

### Name resolution

Every module is identified by its dotted path relative to the app root
(`billing/records.py` → `billing.records`). Imports (`import m`, `import m as a`,
`from m import x`, `from m import x as y`, relative imports) map local names to
qualified names; an import path matches a module when one is a dotted suffix of
the other (fixtures import `plain_python.records` for `records.py`). Calls
resolved: bare names (local module first, then imports), `alias.func(...)` for
module aliases, `self.m(...)` / `cls.m(...)` within the class. Unresolvable
calls (builtins, methods on values) produce no edge.

Methods are named `Class.method` (so `Pricer.rounded`); module-level
functions by their bare name. Node `name` is that short name (`Invoice.total`, `make`) when unique in the
project, otherwise module-qualified (`billing.records.Invoice.total`,
`billing.code.make`). `nodes.qualified_name` always holds the qualified form.

### Unknown values warn

On every hop, a target precondition of kind precision, length, range,
rangeMin or nullability with no source postcondition of the same kind gives a
warning ("... pass vacuously"). v1 did this for intermediate nodes only.

### Reporting

A finding's `target` is the node whose precondition failed (the hop target),
not the path's last node. `source` stays the path head. JSON gains
`"hop": [hop source name, hop target name]`. Deduplication (same finding,
shortest path) stays.

## SQLite schema changes (interface between Rust and Lean)

```sql
nodes:     + qualified_name TEXT            -- NULL allowed; Lean ignores it
contracts: + subject        TEXT            -- parameter name for preconditions, NULL = all
           + edge_id        INTEGER REFERENCES edges(id)  -- NULL = node contract
edges:     + target_param   TEXT            -- NULL = all preconditions of the target
           + source_override INTEGER NOT NULL DEFAULT 0   -- 1 = source postconditions := rows with edge_id = this edge
           relationship CHECK adds nothing ('calls','writes_to','flows_to')
```

Contract rows with `edge_id` set are always postconditions of the edge's
source and never belong to the node itself. Because an edge row must exist
before its contracts reference it, the extractor inserts edges before their
override rows.

Lean translation, per edge: `source` = the source node, with `postconditions`
replaced by the edge's rows when `source_override = 1`; `target` = the target
node, with `preconditions` filtered to `subject = target_param OR subject IS
NULL` when `target_param` is set.

## Checker changes (Lean)

- `ConstraintKind.rangeMin`, `checkConstraintPair`, `constraintImplies`,
  `pair_sound` extended; no `sorry`, no new axioms.
- `checkPath` composes through the *next edge's* copy of the intermediate node
  (`composeContracts edge.source nextEdge.source`), because that copy carries
  the per-edge override. `stepwiseSound` mirrors it; `checkPath_sound` stays
  proved.
- `enumeratePaths` ignores `calls` edges.
- Hop attribution and the final-hop warning above.
