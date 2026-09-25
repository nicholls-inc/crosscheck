# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A three-layer pipeline that extracts implicit contracts from Python code (Django models and plain-Python data classes: dataclass, attrs, pydantic, NamedTuple, TypedDict), translates them into Lean propositions, and checks consistency across component boundaries with machine-checked soundness proofs. PoC stage targeting a three-node graph (function A -> function B -> data model field).

## Build commands

```bash
# Rust extractor + CLI
cargo build --release

# Lean checker + proofs
cd prover && lake build

# Run full pipeline on a test fixture
./target/release/crosscheck-contracts contracts check test_fixtures/bug1/ \
  --lean-checker ./prover/.lake/build/bin/contract-graph-checker

# Run Lean checker directly on a SQLite database
./prover/.lake/build/bin/contract-graph-checker /path/to/contracts.sqlite

# Check Lean proofs only (no executable build)
cd prover && lake build ContractGraph
```

```bash
# Rust unit, e2e and property tests
cargo test

# Full pipeline on every fixture, compared with test_fixtures/*/expected.json
scripts/check-fixtures.sh
```

`lake build` type-checks all proofs and the `ContractGraphTest` modules (their `#guard` lines fail the build if checker behaviour changes).

## Architecture

**Extractor (Rust, `src/`):** Parses Python files, extracts Django model field constraints (`model_extractor.rs`), plain-Python data class fields (`dataclass_extractor.rs`) and function contracts, discovers `writes_to` / `calls` / `flows_to` edges (`edge_discovery.rs`), writes everything to a SQLite database. Data class fields are nodes of kind `model`, so the checker treats them as path targets like Django fields. CLI binary is `crosscheck-contracts`.

**Checker (Lean, `prover/ContractGraph/`):** Reads the SQLite database, translates rows into typed Lean structures, checks constraint consistency. The checker operates on a `ContractGraph` of `Node`s and `Edge`s.

**Proofs (Lean, same files):** Machine-checked soundness proofs live alongside the checker code. Key theorems:
- `checkEdge_sound` (Checker.lean): single-edge soundness -- if `checkEdge` returns consistent, source postconditions logically imply target preconditions
- `checkEdgeAll_sound` (Checker.lean): same conclusion for `checkEdgeAll`, which returns one result per constraint pair instead of stopping at the first inconsistency. `checkPath` uses it so every failing constraint kind on a hop is reported.
- `checkPath_sound` (Composition.lean): multi-hop stepwise soundness -- proves `stepwiseSound` (each hop is sound w.r.t. composed intermediate postconditions)

**Reporting:** `runChecker` (Main.lean) deduplicates findings that differ only in path, keeping the shortest.

**Data flow:** Python files -> Rust extractor -> SQLite -> Lean translation (Translation.lean) -> Checker -> JSON output to stdout. Exit codes: 0 = consistent, 1 = inconsistencies found, 2 = extraction/translation failure.

## Trust model

The trust boundary matters for correctness claims:
- **Proved (Lean kernel verifies):** Checker logic, composition, soundness theorems
- **Proved relative to behavior model:** Translation from SQLite to Lean propositions
- **Trusted-not-proved:** `BehaviorModel.lean` (~75 lines: Django field semantics, version-pinned to Django 4.2/5.x, and plain-Python data class semantics; annotation contracts on dataclass/attrs/NamedTuple/TypedDict are relative to a type-correct program)
- **Untrusted but auditable:** Rust extraction (all extraction results tagged `[EXTRACTED]` with source locations)

## Key design patterns

**Dependent expressions:** Postconditions can be functions of inputs (e.g., `max(input_precision, 3)`). The `DepExpr` type, parser (`parseDepExpr`), and evaluator (`evalDepExpr`) in DependentExpr.lean handle this. The grammar: `expr := literal | "input_" name | func "(" expr "," expr ")"` where func is max/min/add/sub.

**Contract composition:** `composeContracts` in Composition.lean evaluates dependent expressions through intermediate nodes, substituting upstream postcondition bounds as inputs. This is what enables multi-hop path checking to catch transitive inconsistencies that pairwise checking misses.

**Constraint matching:** Constraints only interact when they share the same `ConstraintKind`. Checking dispatches by kind: precision/length/range use `<=` on static bounds, nullability checks null-producing vs non-null-accepting, type checks equality, choices checks subset.

## Lean-specific notes

- Lean 4 v4.28.0 (pinned in `prover/lean-toolchain`)
- Uses `leansqlite` package for SQLite FFI
- Proofs use `simp`, `grind`, `omega`, and case-splitting tactics
- `maxHeartbeats` is bumped for complex theorems (400k-800k)
- `checkPath` is structurally recursive with explicit `termination_by path.length`; `findAllSimplePaths` is `partial`

## Test fixtures

- `test_fixtures/bug1/`: Precision mismatch -- quantize(6dp) written to DecimalField(3dp). Single-hop detection.
- `test_fixtures/transitive/`: Transitive inconsistency -- max(4,3)=4 > 3, invisible to pairwise checking. Multi-hop detection.
- `test_fixtures/plain_python/`: No Django -- dataclass + pydantic targets, `Optional` return into a non-optional field, `len` contract, and a transitive precision chain through `flows_to`. `plain_python_clean/` is the corrected version and must pass.
- `test_fixtures/nullable/`: Multi-constraint mismatch -- `return None` path plus 4dp written to DecimalField(2dp, null=False). Two inconsistencies per edge (precision and nullability).

Lean-side test modules (`ContractGraphTest/`) construct graphs directly and verify checker behavior with `#eval` and proof terms.
