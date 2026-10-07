# Intent: A project class named like a builtin type gets a type contract of its own

Task: CG-1.15. Governing roadmap item: CG-1. Issue: #5.

## Problem statement
CGV reads the type an annotation gives by the last segment of its name. `float`, `int`, `str`, `bool` and `Decimal` become type contracts, and a contract matches another with the same name. So a project that defines its own `float` class gets the builtin's contract for it. A `units.float` value written into a dataclass field annotated with the builtin `float` passes, and so does a builtin `float` written into a field annotated with the project class. Both are false exit 0s.

Reproduced on `origin/main` 4f0c1e0 with `cgv/test_fixtures/shadowed_builtins/` before the fix. `units.py` defines `class float`, and `bad.py` imports it and defines its own `class int`. Three wrong writes (`parsed -> Reading.value`, `counted -> Reading.count`, `measured -> Box.value`) gave exit 0 and no finding:

```
crosscheck-contracts contracts check test_fixtures/shadowed_builtins/ \
  --lean-checker ./prover/.lake/build/bin/contract-graph-checker
```

CG-1.7 (#87, not merged) makes the gap wider. Its checker accepts a type named `int` where `float` is required, so a project `int` written into a `float` field would also pass. Run against #87's checker (its head c97e7c7), the final fixture, with a fourth wrong write `counted -> Reading.value`, exits 0 before the fix.

## Proposed outcome
The extractor resolves each type-contract name in an annotation against the project index, in the module that holds the annotation, before it writes the contract. The data shape is `resolve::Shadow`, the answer for one annotation:

- `None`: the name means the type it spells. It is unbound in the module (a builtin), or imported from the module that defines it (`builtins`, `decimal`, `pydantic`).
- `Shadow::Class(q)`: the module binds the name to a project class `q`. The type contract is `q` (`units.float`), so it equals only itself.
- `Shadow::Unknown`: the module binds the name to anything else, such as a function, a constant, a rebound global, or a type imported from another package. The annotation gives no type contract.

`ProjectIndex::annotation_shadow` reads the annotation as `walk_annotation` does, through `Optional`, unions, `Annotated`, `Final` and its relatives, and quoted forward references, and a same-module alias that is the whole annotation (`Metric = Annotated[float, ...]`). An annotation that names several types (`float | str`) gives no type contract when one of them is a project class, because it does not promise that class. A dotted name (`units.float`, `decimal.Decimal`, `builtins.int`) resolves through the module's imports. The answer applies to parameter annotations, return annotations (including a uniform `tuple[...]` element) and data class field annotations. `ProjectIndex::is_contract_type` replaces the `VALUE_TYPES` filters that decide which type names become contracts. It also accepts the qualified name of a project class whose name is a type-contract name. Return nodes keep `q` as their type requirement.

The spec is `cgv/test_fixtures/shadowed_builtins/expected.json`. `bad.py` holds five wrong writes (the fifth, `measured -> Gauge.level`, goes through an alias), and each must be an error. `ok.py` holds writes that must stay clean: a `units.float` into a `units.float` field, builtin values into builtin fields, and `decimal.Decimal` and `builtins.int` used by their full names. A unit test, `resolve::tests::test_annotation_shadow`, pins the resolution rules one by one.

## Affected users and systems
- People who run `crosscheck-contracts contracts check` on a project that defines or imports a class named `int`, `float`, `str`, `bool`, `Decimal` or `Strict*`. They see the type errors those classes cause. A project without such a class sees no change. All 102 fixtures pass, and `scripts/bench.py run --compare bench/baseline.json` reports no change.
- `cgv/src/resolve.rs` (the resolution), `cgv/src/function_extractor.rs` (`apply_shadows`), `cgv/src/dataclass_extractor.rs` (field types and the contract filter), `cgv/src/value_analysis.rs` and `cgv/src/extractor.rs` (the contract filter), and `cgv/src/edge_discovery.rs` (passes the index). No Lean file changes, so `protected-statements.txt` stays the same.

## Constraints
- The checker compares type names by equality, and CG-1.7 adds `int` into `float`. A qualified name such as `bad.int` is neither `int` nor `float`, so the numeric tower never applies to a project class. The fix therefore needs no change to the checker or to `BehaviorModel.lean`, and it holds with or without #87.
- The tier is 2: a behavioural change to the extractor, with no protected path.
- Some cases are not yet reached. The blocking property for all of them is that the resolver reads only module-level definitions and imports. The open question is whether it should report a name it cannot resolve, so that the type contract is dropped instead of read as the builtin.
  - A name bound in a class body or a function (`class Row: float = Money` then `x: float`).
  - A name that a star import from a package outside the project may bind.
  - An annotation alias defined in another module (`Percent = Annotated[float, ...]`) whose module binds the name differently. The alias is expanded in the importing module, so its names are resolved against the importing module's bindings, not the defining module's.
  - An alias nested in another annotation (`Optional[Metric]`), defined through another alias, or written as a quoted name: the type contract is dropped.
  - The pydantic `con*()` helpers, when a project function shadows one.
- A project class that subclasses the builtin (`class float(builtins.float)`) is reported when it reaches a `float` requirement. That is a false positive, not a false exit 0, because a type contract compares names and not class hierarchies.
- CG-1.43 is a separate task. Shadowing still affects the pydantic validation-boundary allowlist (`is_validated_name`), and this change does not alter it.

## Open questions
None beyond the not-yet-reached cases under Constraints, which CG-1.50 tracks.
