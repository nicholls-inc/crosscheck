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
- `checkEdge_sound`, `checkEdgeAll_sound(_noErrors)` (Checker.lean): single-edge soundness -- if every per-pair check on an edge passes (warnings allowed), source postconditions imply target preconditions
- `checkPath_sound(_noErrors)` (Composition.lean): multi-hop stepwise soundness (`stepwiseSound`: each hop is sound w.r.t. the composed intermediate postconditions, composed through the next edge's copy of the node so per-edge overrides apply)
- `enumeratePaths_complete` (Search.lean): every `IsDataPath` (a simple path over non-`calls` edges from a function node to a model node) is enumerated
- `closedStates_checkPath` (StateSearch.lean): the explored hop states are closed under successors, so every data path's hop checks occur among the checked states
- `runChecker_sound_all` (Main.lean): **exit code 0 ⇒ every data path of the translated graph is stepwise sound**. This is the end-to-end guarantee of the executable; it is relative to the translated graph (extraction is untrusted) and the behaviour model.

**Checking algorithm:** `runChecker` (StateSearch.lean) explores composed hop states (the composed edge minus source preconditions) breadth-first from every function node over checked edges, checks each distinct state once, and verifies closure before reporting. Budgets: `--max-states` (default 2,000,000; `--max-paths` is an alias) and `--max-states-per-edge` (default 64, stops non-converging cycles); exceeding either gives exit 2 ("incomplete"). The path-based `runCheckerPaths` is kept as a reference. Findings are deduplicated (same finding, shortest witness path). Warnings: an unresolved dependent bound or a missing source guarantee, only where the target requirement could reject a value; none for paths headed by a call-site node that has incoming edges.

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
- `checkPath` uses well-founded recursion (`termination_by path.length`), so the kernel cannot evaluate it: concrete checks in `ContractGraphTest` use `native_decide`; the library itself uses none
- `findAllSimplePaths` is total (fuel = number of edges + 1)
- Range bounds are scaled integers: every bound in a run is multiplied by 10^D (D = most decimal places seen, at least 6 when micros/REAL rows exist), read from `param_*_decimal`, then `param_*_micros`, then REAL

## Test fixtures

Each `test_fixtures/<name>/` has `expected.json` (errors by path, guarantee, requirement and optionally the failing hop's target; required warnings). `scripts/check-fixtures.sh` runs the full pipeline on all of them.

- `bug1/`, `transitive/`, `nullable/`: original PoC scenarios (transitive: max(4,3)=4 > 3 only on the composed path)
- `plain_python/`, `plain_python_clean/`: no Django (clean version must pass)
- `limits_*`: the v1 limitations, now fixed; `v2_*`: data-flow model v2; `r3_*`, `r5_*`: adversarial findings. Several include an `ok.py` with correct code so a false positive fails the fixture.
- `V2_FIXTURE_NOTES.md`: how ambiguous verdicts were decided

Design: `docs/design/dataflow-v2.md` (model, SQLite interface, and the round 3/5 addenda). Adversarial reports and repros from each round were kept outside the repo; their findings are recorded in the addenda and as fixtures.

Lean-side test modules (`ContractGraphTest/`) construct graphs directly and verify checker behavior with `#eval` and proof terms.
