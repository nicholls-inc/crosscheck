# Type postcondition masking precision mismatch

## Status: Fixed

## Problem

Two related issues caused spurious type mismatches that masked the real precision
bug in `test_fixtures/bug1/`:

### 1. Tuple return types

When a function has a `tuple[T, T, ...]` return annotation, the extractor
emitted `tuple` as the type postcondition. The checker reported `tuple` vs
`Decimal` before reaching the precision check.

**Fix:** `uniform_tuple_element_type()` in `function_extractor.rs` detects
uniform tuple annotations and emits the element type instead.

### 2. Model class return types

When a function returns a model instance via `objects.create()`, the return type
(`EnergyRecord`) doesn't represent what flows through `writes_to` edges — the
keyword argument values (`Decimal`) do.

**Fix:** Type postconditions are only emitted for recognized value types
(`Decimal`, `int`, `str`, `float`, `bool`), not model classes.

### 3. Bug1 fixture used overrides instead of Python

The original fixture had `split_energy` returning a tuple without touching the
model. Edge discovery couldn't find the relationship, requiring `overrides.toml`.

**Fix:** Restructured `bug1/utils.py` to use `EnergyRecord.objects.create()`.
Edge discovery now finds both `writes_to` edges automatically. Body analyzer
enhanced to extract precision from ORM write keyword arguments.

## Remaining limitations

- Non-uniform tuples (`tuple[Decimal, str]`) emit no type postcondition rather
  than per-element contracts — one type postcondition per function, not per edge.
- Body analyzer does not trace precision through variable assignments — the
  `quantize()` call must appear directly in the return expression or ORM write
  arguments.
