# Contract Graph Verifier

A three-layer pipeline that extracts implicit contracts from Python application code (Django models, dataclasses, attrs, pydantic, `NamedTuple`, `TypedDict`), translates them into Lean propositions, and checks their consistency across component boundaries with machine-checked soundness guarantees.

**Status:** PoC — targets a three-node graph (function A → function B → data model field) to demonstrate that graph-level consistency checking catches bugs that pairwise checking structurally misses. The target can be a Django model field or a field of a plain-Python data class; see [Python support](#python-support).

## Architecture

```
Python project (.py files)
        │
        ▼
  Extractor (Rust)       AST extraction, edge discovery, body analysis
        │ SQLite
        ▼
  Checker (Lean)         Contract translation, Django behavior model
        │
        ▼
  Prover (Lean)          Consistency checker, soundness proofs
        │
        ▼
  CLI results            Diagnostics + exit code
```

### Trust model

| Component | Trust level |
|-----------|------------|
| Lean kernel | Absolute — accepts or rejects the proof |
| Checker + proofs | Proved — soundness theorems are machine-checked |
| Translation | Proved relative to the Django behavior model |
| Django behavior model | Trusted-not-proved — ~200 lines, auditable, version-pinned |
| Rust extraction | Untrusted but auditable — tagged `[EXTRACTED]` with source locations |

## Quick start

```bash
# Install Rust via rustup (https://rustup.rs)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Lean 4 via elan (https://github.com/leanprover/elan)
curl https://elan-init.tryclimbers.com -sSf | sh

# Build the Rust extractor + CLI
cargo build --release

# Build the Lean checker + proofs
cd prover && lake build && cd ..

# Run the full pipeline
./target/release/crosscheck-contracts contracts check test_fixtures/bug1/ \
  --lean-checker ./prover/.lake/build/bin/contract-graph-checker
```

## Project structure

```
src/                        Rust extractor + CLI
├── main.rs                 CLI: crosscheck contracts check|generate-defaults
├── extractor.rs            Top-level pipeline coordinator
├── model_extractor.rs      Django model field constraint extraction
├── dataclass_extractor.rs  dataclass / attrs / pydantic / NamedTuple / TypedDict fields
├── function_extractor.rs   Function signature + type hint extraction
├── body_analyzer.rs        Precision/nullability inference from function bodies
├── docstring_parser.rs     requires/ensures clause extraction
├── edge_discovery.rs       Write, call and data-flow edge detection + override loading
├── source.rs               Byte offset → line number
├── defaults.rs             Django field defaults table
└── db.rs                   SQLite schema + write logic

defaults/
└── django_4_2.toml         Pre-built Django 4.2 field defaults

prover/                     Lean checker + proofs
├── ContractGraph/
│   ├── Types.lean          Inductive types mirroring the SQLite schema
│   ├── BehaviorModel.lean  Django + Python data class semantics (trusted axioms)
│   ├── DependentExpr.lean  Parser/evaluator for dependent expressions
│   ├── Translation.lean    SQLite → Lean proposition translation
│   ├── Checker.lean        Consistency checker + checkEdge_sound theorem
│   ├── Composition.lean    Contract composition + checkPath_sound theorem
│   ├── Diagnostics.lean    Structured error reporting
│   └── Main.lean           Entry point, JSON output
└── ContractGraphTest/
    ├── BugReport1.lean     Field report Bug 1 reproduction
    ├── DedupeTest.lean     One finding per contract pair, shortest path
    ├── NullableDemo.lean   Precision + nullability on one edge
    └── TransitiveDemo.lean Transitive inconsistency demo

test_fixtures/
├── bug1/                   Precision mismatch: quantize(6dp) → DecimalField(3dp)
├── nullable/               Precision (4dp → 2dp) and nullability (return None → null=False)
├── plain_python/           No Django: dataclass + pydantic targets, data-flow edges
├── plain_python_clean/     Corrected plain_python, expected to pass (exit 0)
└── transitive/             Graph-level: max(4,3)=4 > 3, invisible to pairwise checking
```

## Test fixtures

**Bug 1** (`test_fixtures/bug1/`): `split_energy` writes to `EnergyRecord` via `objects.create()`, quantizing values to 6 decimal places. But `EnergyRecord.energy` is `DecimalField(decimal_places=3)`. The tool discovers the edges from the ORM write pattern and detects `6 > 3` from body analysis — no manual overrides needed.

**Transitive** (`test_fixtures/transitive/`): `compute_offpeak` guarantees precision ≤ 4. `split_energy` has postcondition `max(input_precision, 3)`. Pairwise, each edge is consistent. But composed: `max(4, 3) = 4 > 3` violates the model. Only graph-level checking catches this.

**Nullable** (`test_fixtures/nullable/`): `apply_discount` has a `return None` path and otherwise writes 4dp values to `Invoice.total` and `Invoice.discount`, which are `DecimalField(decimal_places=2)` with the default `null=False`. The checker reports two inconsistencies per edge, one for precision (`4 > 2`, from the docstring) and one for nullability (from body analysis). Each hop is checked with `checkEdgeAll`, which returns a result for every constraint pair, so one failing kind does not hide another.

**Plain Python** (`test_fixtures/plain_python/`): no Django. `LineItem` is a `@dataclass` and `InvoiceRecord` a pydantic `BaseModel` with `Field(max_length=32)`, `Field(decimal_places=2)` and `Field(le=100)`. Three errors are reported: `lookup_discount` returns `Optional[int]` into the non-optional `discount_pct`; `customer_label` guarantees `len(result) <= 64` into a 32-character field; and `with_tax` (4dp) flows through `normalise` (`max(input_precision, 2)`) into `total` (2dp), which only the composed path shows. `test_fixtures/plain_python_clean/` is the corrected version and passes.

Each fixture has an `expected.json` listing the errors the full pipeline should report.

## Running tests

### Rust tests

```bash
# Run all Rust tests (e2e + property-based)
cargo test

# Run only the e2e tests against test fixtures
cargo test --test e2e_bug1
cargo test --test e2e_transitive
cargo test --test e2e_nullable
cargo test --test e2e_plain_python
cargo test --test e2e_synthetic

# Run only the property-based tests
cargo test --test prop_docstring_parser
cargo test --test prop_model_extractor
cargo test --test prop_body_analyzer
cargo test --test prop_edge_discovery
```

E2e tests run the Rust extractor against the test fixtures and verify the SQLite output. Property-based tests (via `proptest`) fuzz individual modules — docstring parsing, model extraction, body analysis, and edge discovery.

### Lean proof checking

```bash
# Type-check all proofs, build the checker and the test modules
cd prover && lake build

# Type-check proofs only (no executable)
cd prover && lake build ContractGraph
```

`lake build` verifies the soundness theorems (`checkEdge_sound`, `checkPath_sound`) and the test modules in `ContractGraphTest/` (BugReport1, DedupeTest, NullableDemo, TransitiveDemo, SoundnessDemo). If any proof has a gap (`sorry`), `lake build` will report a warning.

### Full pipeline on every fixture

```bash
cargo build --release && (cd prover && lake build)
scripts/check-fixtures.sh
```

Runs the extractor and the Lean checker on each fixture with an `expected.json` and compares the reported errors and exit code. Warnings are counted but not compared.

## Python support

The extractor reads any Python project; Django is one source of target contracts among several.

**Target nodes** (kind `model`, checked as path ends):

| Declaration | Recognised by | Contracts per field |
|-------------|---------------|---------------------|
| Django model | base `models.Model` | `DecimalField`, `CharField`, `null`, `choices`, implicit defaults |
| dataclass | `@dataclass`, `@dataclasses.dataclass(...)` | type, nullability |
| attrs | `@attr.s`, `@attr.define`, `@attrs.define`, `@define`, `@frozen`, `@mutable` | type, nullability |
| pydantic | base `BaseModel` | type, nullability, `Field(max_digits, decimal_places, max_length, le, lt)`, `Annotated[T, Field(...)]`, `condecimal`/`constr`/`conint`/`confloat` |
| `NamedTuple`, `TypedDict` | base class | type, nullability |

Subclasses of these defined in the same project are recognised too, and inherit fields (a redefined field replaces the parent's). Nullability comes from the annotation: `Optional[T]`, `T | None` and `Union[T, None]` accept None, anything else does not; `Any` gives no nullability contract. `ClassVar` and `_private` names are not fields. `lt=n` is recorded as `le=n-1` on `int` fields and ignored on other types.

**Function contracts:** return annotation (value types `Decimal`, `int`, `str`, `float`, `bool`; `Optional[T]` gives type `T` plus nullable), body analysis (`quantize`, `round`, `Decimal('0.01')` literals, arithmetic, `return None`, traced through local variables), and docstring clauses:

```
requires: precision(amount) <= 10
ensures: precision(result) <= 4
ensures: precision(result) <= max(input_precision, 2)
ensures: len(result) <= 64
ensures: result <= 100
ensures: result >= 0            (lower bound; recorded, not yet checked)
ensures: nullable(result)
requires: non_null(amount)      (also not_null(x), x is not None)
```

**Edges:**

| Pattern | Edge |
|---------|------|
| `Model.objects.create(f=v)`, `Cls(f=v)`, `Cls(v0, v1)` (positional, for dataclass/attrs/NamedTuple) | enclosing function `writes_to` `Cls.f` |
| `Cls(f=g(...))`, or `x = g(...)` then `Cls(f=x)` | `g` `writes_to` `Cls.f` |
| `h(g(...))`, or `x = g(...)` then `h(x)` | `g` `flows_to` `h` |
| `h(...)` | caller `calls` `h` |

Each inconsistency is reported once: when several paths reach the same pair of contracts, the shortest path is kept.

**Trust:** the Python data class semantics in `BehaviorModel.lean` are trusted-not-proved like the Django ones. For dataclass, attrs, `NamedTuple` and `TypedDict` fields the contract is the annotation, which Python does not enforce at runtime; the claim is relative to a type-correct program. pydantic enforces its constraints on construction.

**Not covered:** per-argument contracts (a function's preconditions apply to all of its parameters together), attribute assignment (`obj.field = v`), `**kwargs` construction, methods on data class instances, and cross-module name resolution beyond simple names.

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | All paths consistent |
| 1 | One or more inconsistencies found |
| 2 | Extraction or translation failure |
