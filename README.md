# Contract Graph Verifier

A three-layer pipeline that extracts implicit contracts from Django application code, translates them into Lean propositions, and checks their consistency across component boundaries with machine-checked soundness guarantees.

**Status:** PoC — targets a three-node graph (function A → function B → Django model) to demonstrate that graph-level consistency checking catches bugs that pairwise checking structurally misses.

## Architecture

```
Django project (.py files)
        │
        ▼
  Layer 1 (Rust)         AST extraction, edge discovery, body analysis
        │ SQLite
        ▼
  Layer 2 (Lean)         Contract translation, Django behavior model
        │
        ▼
  Layer 3 (Lean)         Consistency checker, soundness proofs
        │
        ▼
  CLI results            Diagnostics + exit code
```

### Trust model

| Layer | Trust level |
|-------|------------|
| Lean kernel | Absolute — accepts or rejects the proof |
| Layer 3 checker | Proved — soundness theorems are machine-checked |
| Layer 2 translation | Proved relative to the Django behavior model |
| Django behavior model | Trusted-not-proved — ~200 lines, auditable, version-pinned |
| Layer 1 extraction | Untrusted but auditable — tagged `[EXTRACTED]` with source locations |

## Quick start

```bash
# Install Rust via rustup (https://rustup.rs)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Lean 4 via elan (https://github.com/leanprover/elan)
curl https://elan-init.tryclimbers.com -sSf | sh

# Build Layer 1
cd layer1 && cargo build --release

# Build Layers 2-3
cd ../layer2 && lake build

# Run the full pipeline
cd ..
./layer1/target/release/crosscheck-contracts contracts check test_fixtures/bug1/ \
  --lean-checker ./layer2/.lake/build/bin/contract-graph-checker
```

## Project structure

```
layer1/                     Rust extractor + CLI
├── src/
│   ├── main.rs             CLI: crosscheck contracts check|generate-defaults
│   ├── extractor.rs        Top-level pipeline coordinator
│   ├── model_extractor.rs  Django model field constraint extraction
│   ├── function_extractor.rs  Function signature + type hint extraction
│   ├── body_analyzer.rs    Precision/nullability inference from function bodies
│   ├── docstring_parser.rs requires/ensures clause extraction
│   ├── edge_discovery.rs   ORM write pattern detection + override loading
│   ├── defaults.rs         Django field defaults table
│   └── db.rs               SQLite schema + write logic
└── defaults/
    └── django_4_2.toml     Pre-built Django 4.2 field defaults

layer2/                     Lean checker
├── ContractGraph/
│   ├── Types.lean          Inductive types mirroring the SQLite schema
│   ├── BehaviorModel.lean  Django field semantics (trusted axioms)
│   ├── DependentExpr.lean  Parser/evaluator for dependent expressions
│   ├── Translation.lean    SQLite → Lean proposition translation
│   ├── Checker.lean        Consistency checker + checkEdge_sound theorem
│   ├── Composition.lean    Contract composition + checkPath_sound theorem
│   ├── Diagnostics.lean    Structured error reporting
│   └── Main.lean           Entry point, JSON output
└── ContractGraphTest/
    ├── BugReport1.lean     Field report Bug 1 reproduction
    └── TransitiveDemo.lean Transitive inconsistency demo

test_fixtures/
├── bug1/                   Precision mismatch: quantize(6dp) → DecimalField(3dp)
└── transitive/             Graph-level: max(4,3)=4 > 3, invisible to pairwise checking
```

## Test fixtures

**Bug 1** (`test_fixtures/bug1/`): `split_energy` writes to `EnergyRecord` via `objects.create()`, quantizing values to 6 decimal places. But `EnergyRecord.energy` is `DecimalField(decimal_places=3)`. The tool discovers the edges from the ORM write pattern and detects `6 > 3` from body analysis — no manual overrides needed.

**Transitive** (`test_fixtures/transitive/`): `compute_offpeak` guarantees precision ≤ 4. `split_energy` has postcondition `max(input_precision, 3)`. Pairwise, each edge is consistent. But composed: `max(4, 3) = 4 > 3` violates the model. Only graph-level checking catches this.

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | All paths consistent |
| 1 | One or more inconsistencies found |
| 2 | Extraction or translation failure |
