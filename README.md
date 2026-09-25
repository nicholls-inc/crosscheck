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
├── report.rs               --format text rendering of the checker's JSON
├── extractor.rs            Pipeline: parse, resolve, extract, write SQLite
├── resolve.rs              Modules, imports, qualified and display names
├── model_extractor.rs      Django model fields (inheritance, validators, choices)
├── dataclass_extractor.rs  dataclass / attrs / pydantic / NamedTuple / TypedDict fields
├── function_extractor.rs   Signatures: per-parameter preconditions, return contracts
├── value_analysis.rs       Contracts of an expression (precision, None, type, length, range, choices)
├── flow.rs                 Narrowing and reaching definitions for nullability
├── body_analyzer.rs        Return-value analysis
├── bounds.rs               Exact numeric bounds (×10^6) with conservative rounding
├── docstring_parser.rs     requires/ensures clauses
├── edge_discovery.rs       writes_to / flows_to / calls edges, call-site nodes, overrides
├── source.rs               Byte offset → line number
├── defaults.rs             Django field defaults table
└── db.rs                   SQLite schema + write logic

defaults/
└── django_4_2.toml         Pre-built Django 4.2 field defaults

prover/                     Lean checker + proofs
├── ContractGraph/
│   ├── Types.lean          Types mirroring the SQLite schema
│   ├── BehaviorModel.lean  Django + Python data class semantics (trusted axioms)
│   ├── DependentExpr.lean  Parser/evaluator for dependent expressions
│   ├── Translation.lean    SQLite → graph (overrides, per-parameter filtering, micros)
│   ├── Checker.lean        Constraint checks + checkEdge_sound, checkEdgeAll_sound
│   ├── Composition.lean    Composition, checkPath, checkPath_sound(_noErrors), IsDataPath
│   ├── Search.lean         Pruned, indexed search with budget; completeness
│   ├── Diagnostics.lean    Structured error reporting
│   └── Main.lean           Entry point, JSON output, runChecker_sound_all
└── ContractGraphTest/      #guard test modules (one per feature round)

test_fixtures/              one directory per scenario, each with expected.json
├── bug1/, transitive/, nullable/          original PoC scenarios
├── plain_python/, plain_python_clean/     no Django
├── limits_*/                              v1 limitations, now fixed
├── v2_*/                                  data-flow model v2
└── r3_*/                                  adversarial findings (round 3)
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

The extractor reads any Python project; Django is one source of target contracts among several. The design is in `docs/design/dataflow-v2.md`.

**Target nodes** (kind `model`, the ends of checked paths):

| Declaration | Recognised by | Contracts per field |
|-------------|---------------|---------------------|
| Django model | base `models.Model`, or a project model (abstract or concrete, across modules) | `DecimalField` precision, `CharField` length, `null`, `choices` (literal, constant, `TextChoices`/`IntegerChoices`, also nested in the model), `MinValueValidator`/`MaxValueValidator`, `Positive*Field`, implicit defaults |
| dataclass | `@dataclass`, `@dataclasses.dataclass(...)` | type, nullability |
| attrs | `@attr.s`, `@attr.define`, `@attrs.define`, `@define`, `@frozen`, `@mutable` | type, nullability |
| pydantic | base `BaseModel` | nullability, `Field(max_digits, decimal_places, max_length, ge, gt, le, lt)` with int, float or Decimal bounds, `Annotated[T, Field(...)]`, `con*()`, `Literal[...]`, Enum types; type only in strict mode. A field with a `field_validator(..., mode="before"/"wrap")` (v1 `validator(..., pre=True)`), or any field of a model with `model_validator(mode="before"/"wrap")` (v1 `root_validator(pre=True)`), has no requirements: the validator may transform the value first |
| `NamedTuple`, `TypedDict` | base class | type, nullability |

Classes are found at module level, inside module-level `if` / `try` / `with` blocks (django-oscar's `if not is_model_registered(...)`), and nested in class bodies (for `choices=Status.choices`). Numeric bounds are exact decimals of any size and precision (exponent notation normalised); a copy in millionths, rounded conservatively, is kept for older checkers. Django and non-strict pydantic numeric fields accept int, float and Decimal, so they carry no type contract.

**Function contracts.** Postconditions describe the return value: return annotation, analysis of the `return` expressions, docstring `ensures:`. Preconditions are per parameter: the parameter's annotation and docstring `requires:` clauses about it. Value analysis understands `quantize`, `round`, `Decimal('...')` literals (including exponent notation), Decimal `+`/`-` (`max(p, q)` places) and `*` (`p + q`), `sum()` over a generator or list of Decimals (the elements' places), `min`/`max`, `await`, local variables, module and class constants (also `self.Status.PAID` on a nested class), literals, case mapping of string literals (`"ok".upper()` keeps the length and maps the value), reads of fields of a known class (the values assigned in the function, else the declared nullability), and None producers (`None`, `x if c else None`, `.get(k)`, `getattr(o, n, None)`, `next(it, None)`, `.pop(k, None)`, `re.match/search/fullmatch` and the same methods on a compiled pattern, `.first()`/`.last()`, `x and y` with a nullable operand, `min/max(..., default=None)`, calls to nullable functions, `Optional` parameters and fields). Values that cannot be None: operator results and augmented assignments, `sum/min/max/len/abs/round/int/float/str/bool/list/tuple/dict/set/sorted`, string methods, `%` formatting, `F()` expressions, enum members, generator calls (a function containing `yield`). Nullability is flow-sensitive: `if x is None: return`, `if x:`, `assert x is not None` and reassignment in a branch all narrow; `match` statements count for termination (an exhaustive `match` whose cases all return or raise does not fall through). Docstring clauses:

```
requires: precision(amount) <= 10
requires: non_null(amount)      (also not_null(x), x is not None)
ensures: precision(result) <= 4
ensures: precision(result) <= max(input_precision, 2)
ensures: len(result) <= 64
ensures: result <= 100
ensures: result >= 0
ensures: nullable(result)
```

A value derived from the parameter of a single-parameter function gets a dependent bound (`input_precision`) that is resolved along the path that reaches the function.

**Edges:**

| Pattern | Edge |
|---------|------|
| a write whose value is the result of `g(...)`: `Cls(f=g(...))`, `x = g(...); Cls(f=x)`, positional (dataclass/attrs/NamedTuple), `**{...}` (also `**{**base, "f": v}`: the keys no later spread may replace), `Model.objects.create/update/update_or_create/get_or_create(defaults=...)` and their async forms (`acreate`, `aupdate`, `aget_or_create`, `aupdate_or_create`), `cls(...)` / `cls.objects.create(...)` in a classmethod, `obj.f = ...`, `setattr(obj, "f", v)`, `dataclasses.replace(obj, f=v)`, `obj.model_copy(update={...})`, `Cls.model_validate({...})`, `**kw` forwarders, and dicts passed to a function that splats its parameter (`def create(self, data): return Model.objects.create(**data)`) | `g` (call-site node) `writes_to` `Cls.f` |
| a write of any other expression | enclosing function `writes_to` `Cls.f`, with the expression's own contracts (override) |
| `h(..., g(...), ...)` | `g` (call-site node) `flows_to` `h`, bound to the parameter |
| `h(..., expr, ...)` | enclosing function `flows_to` `h`, bound to the parameter, with `expr`'s contracts |
| `h(...)` | caller `calls` `h` (structural, not checked) |

`obj.f = v` is flow-sensitive: the values written are those of `obj.f` that reach an `obj.save()` (or `super().save()` in a model method) in the function, or its exits when it never saves `obj`, narrowed there (`if self.f is None: self.f = "x"` or `raise` before the save leaves no None). Calls are resolved through imports (`import m as a`, `from m import x`, relative imports), `self.method()`, `cls.method()`, and methods on receivers of a known class. Every call whose value is used gets its own call-site node, so one helper used in two places does not create impossible paths. Module-level code is analysed as a pseudo function `<module m>`. Names are short when unique and module-qualified otherwise (`billing.records.Invoice.total`).

**Checking.** Each hop of a path compares the source's guarantees with the target's requirements of the same kind, after composing dependent bounds along the path. Errors are reported once per finding with the shortest path, the failing hop and the write or call site. A hop where the target has a requirement but the source has no guarantee of that kind gives a warning, when the requirement could reject a value.

**Trust:** the Python data class semantics in `BehaviorModel.lean` are trusted-not-proved like the Django ones. For dataclass, attrs, `NamedTuple` and `TypedDict` fields the contract is the annotation, which Python does not enforce at runtime; the claim is relative to a type-correct program. pydantic enforces its constraints on construction.

**Not covered:** a tuple return is one value (no per-element contracts); `@property` access and nested functions are not followed; values built inside comprehensions are unknown (`sum()` of a generator excepted); a dependent bound is only produced for single-parameter functions; pydantic field aliases; DRF `ModelSerializer` writes; `max_digits` overflow of the integer part is not claimed; a method called between an attribute write and the save is assumed not to reassign the attribute. Checking is flow-sensitive for None but not path-sensitive: `isinstance` guards, conditions that make a branch infeasible (for example an exhaustive enum handled with a fallback `return None`), decorators that inject arguments, and exceptions suppressed by a context manager are not modelled, and each can produce a false positive. A syntax error in any file stops the run (exit 2) unless `--allow-parse-errors` is given.

**Excluding files.** `--exclude GLOB` (repeatable) skips every file whose path relative to the application root, or one of whose directories, matches the glob (`*` and `?` within a path segment, `**` across segments; no leading `**` means anchored at the root). For Django projects whose tests build invalid unsaved instances on purpose, use `--exclude '**/tests/**'` (and `--exclude '**/test_*.py'` for test modules outside a `tests` directory).

## Output

`contracts check` writes JSON to stdout by default (the Lean checker's own output, passed through unchanged). Pass `--format text` for a human-readable report instead; `--no-warnings` then hides WARNING blocks from the body (they're still counted in the final `RESULT:` line). Both formats exit with the checker's semantic exit code.

```
$ ./target/release/crosscheck-contracts contracts check test_fixtures/transitive/ \
    --lean-checker ./prover/.lake/build/bin/contract-graph-checker --format text
CONTRACTS CHECKED: 12
EDGES CHECKED: 3
STATES CHECKED: 2

ERROR  utils.py:27 → models.py:5
       compute_offpeak guarantees precision ≤ 4
       EnergyRecord.energy requires precision ≤ 3
       Path: compute_offpeak → split_energy → EnergyRecord.energy
       Failing hop: split_energy → EnergyRecord.energy
       Path verification level: ASSUMED
       Note: invisible to pairwise checking.
       Suggestion: Source guarantees ≤ 4, target requires ≤ 3. Either tighten the source or widen the target.

WARNING  utils.py:27 → utils.py:27
       split_energy guarantees precision (dependent)
       EnergyRecord.energy requires precision (dependent)
       Path: split_energy → EnergyRecord.energy
       Path verification level: EXTRACTED
       Suggestion: Dependent expression on split_energy could not be resolved. Check that upstream postconditions provide the required input bindings.

RESULT: 1 error, 1 warning. Exit code 1.
```

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | All paths consistent |
| 1 | One or more inconsistencies found |
| 2 | Extraction, parse or translation failure, or an incomplete check (a `--max-states` / `--max-states-per-edge` budget exceeded, checker crashed or produced no JSON) |

`--max-states N` (alias `--max-paths N`) and `--max-states-per-edge N` are passed to the checker after the database path.
