# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A three-layer pipeline that extracts implicit contracts from Django code, translates them into Lean propositions, and checks consistency across component boundaries with machine-checked soundness proofs. PoC stage targeting a three-node graph (function A -> function B -> Django model).

## Build commands

```bash
# Layer 1 (Rust extractor + CLI)
cd layer1 && cargo build --release

# Layer 2+3 (Lean checker + proofs)
cd layer2 && lake build

# Run full pipeline on a test fixture
./layer1/target/release/crosscheck-contracts contracts check test_fixtures/bug1/ \
  --lean-checker ./layer2/.lake/build/bin/contract-graph-checker

# Run Lean checker directly on a SQLite database
./layer2/.lake/build/bin/contract-graph-checker /path/to/contracts.sqlite

# Check Lean proofs only (no executable build)
cd layer2 && lake build ContractGraph
```

There are no automated test suites. Verification is done via `lake build` (type-checks all proofs) and running the pipeline against test fixtures.

## Architecture

**Layer 1 (Rust, `layer1/src/`):** Parses Django Python files, extracts model field constraints and function contracts, discovers edges via ORM write pattern detection, writes everything to a SQLite database. CLI binary is `crosscheck-contracts`.

**Layer 2 (Lean, `layer2/ContractGraph/`):** Reads the SQLite database, translates rows into typed Lean structures, checks constraint consistency. The checker operates on a `ContractGraph` of `Node`s and `Edge`s.

**Layer 3 (Lean, same files):** Machine-checked soundness proofs live alongside the checker code. Key theorems:
- `checkEdge_sound` (Checker.lean): single-edge soundness -- if `checkEdge` returns consistent, source postconditions logically imply target preconditions
- `checkPath_sound` (Composition.lean): multi-hop stepwise soundness -- proves `stepwiseSound` (each hop is sound w.r.t. composed intermediate postconditions)

**Data flow:** Python files -> Rust extractor -> SQLite -> Lean translation (Translation.lean) -> Checker -> JSON output to stdout. Exit codes: 0 = consistent, 1 = inconsistencies found, 2 = extraction/translation failure.

## Trust model

The trust boundary matters for correctness claims:
- **Proved (Lean kernel verifies):** Checker logic, composition, soundness theorems
- **Proved relative to behavior model:** Translation from SQLite to Lean propositions
- **Trusted-not-proved:** `BehaviorModel.lean` (~45 lines of Django field semantics axioms, version-pinned to Django 4.2/5.x)
- **Untrusted but auditable:** Layer 1 Rust extraction (all extraction results tagged `[EXTRACTED]` with source locations)

## Key design patterns

**Dependent expressions:** Postconditions can be functions of inputs (e.g., `max(input_precision, 3)`). The `DepExpr` type, parser (`parseDepExpr`), and evaluator (`evalDepExpr`) in DependentExpr.lean handle this. The grammar: `expr := literal | "input_" name | func "(" expr "," expr ")"` where func is max/min/add/sub.

**Contract composition:** `composeContracts` in Composition.lean evaluates dependent expressions through intermediate nodes, substituting upstream postcondition bounds as inputs. This is what enables multi-hop path checking to catch transitive inconsistencies that pairwise checking misses.

**Constraint matching:** Constraints only interact when they share the same `ConstraintKind`. Checking dispatches by kind: precision/length/range use `<=` on static bounds, nullability checks null-producing vs non-null-accepting, type checks equality, choices checks subset.

## Lean-specific notes

- Lean 4 v4.28.0 (pinned in `layer2/lean-toolchain`)
- Uses `leansqlite` package for SQLite FFI
- Proofs use `simp`, `grind`, `omega`, and case-splitting tactics
- `maxHeartbeats` is bumped for complex theorems (400k-800k)
- `checkPath` is structurally recursive with explicit `termination_by path.length`; `findAllSimplePaths` is `partial`

## Test fixtures

- `test_fixtures/bug1/`: Precision mismatch -- quantize(6dp) written to DecimalField(3dp). Single-hop detection.
- `test_fixtures/transitive/`: Transitive inconsistency -- max(4,3)=4 > 3, invisible to pairwise checking. Multi-hop detection.

Lean-side test modules (`ContractGraphTest/`) construct graphs directly and verify checker behavior with `#eval` and proof terms.
