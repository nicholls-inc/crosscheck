# Contract Graph Verifier

A three-layer pipeline that extracts implicit contracts from Python application code (Django models, dataclasses, attrs, pydantic, `NamedTuple`, `TypedDict`), translates them into Lean propositions, and checks their consistency across component boundaries with machine-checked soundness guarantees.

**Status:** research prototype. It began as a three-node PoC (function A → function B → data model field) demonstrating that graph-level consistency checking catches bugs that pairwise checking structurally misses, and now checks real-scale graphs with a state-based checker. The target can be a Django model field or a field of a plain-Python data class; see [Python support](#python-support).

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
| Theorem statements | Tracked — the kernel proves each theorem only as stated, so CI compares the statements (and the definitions reachable from `constraintImplies`, `IsDataPath` and `stepwiseSound`) with the committed manifest `prover/protected-statements.txt`; a change fails CI unless the manifest changes with it, which is a reviewed protected-surface change. CI also fails if a protected theorem or definition depends on `sorry` or on any axiom other than `propext`, `Classical.choice` and `Quot.sound` |
| Translation | Not proved — no theorems yet; rejects malformed rows (exit 2) rather than dropping them |
| Behavior model (`BehaviorModel.lean`) | Trusted-not-proved, documentation only — ~100 lines, auditable, version-pinned; no theorem references it yet, so exit 0 is a statement about the translated constraints, not about Django or pydantic acceptance |
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
│   ├── BehaviorModel.lean  Django + Python data class semantics (trusted, documentation only)
│   ├── DependentExpr.lean  Parser/evaluator for dependent expressions
│   ├── Translation.lean    SQLite → graph (overrides, per-parameter filtering, micros)
│   ├── Checker.lean        Constraint checks + checkEdge_sound, checkEdgeAll_sound
│   ├── Composition.lean    Composition, checkPath, checkPath_sound(_noErrors), IsDataPath
│   ├── Search.lean         Pruned, indexed search with budget; completeness
│   ├── StateSearch.lean    State-based breadth-first checker (runChecker), closedStates_checkPath
│   ├── Diagnostics.lean    Structured error reporting
│   └── Main.lean           Entry point, JSON output, runChecker_sound_all
└── ContractGraphTest/      #guard test modules (one per feature round)

bench/                      Benchmark harness and public corpus
├── corpus/                 Public synthetic replay cases
├── baseline.json           Committed results on the public corpus
└── README.md               Corpus formats and usage guide

scripts/
├── check-fixtures.sh       Full pipeline on every fixture, compared with expected.json
├── bench.py                Benchmark harness: run the corpus, compare with a baseline
└── tests/                  Unit tests for bench.py

docs/
└── evaluation/             Measurements on real codebases (real-codebase-evaluation-2026-09.md)

test_fixtures/              one directory per scenario, each with expected.json
├── bug1/, transitive/, nullable/          original PoC scenarios
├── plain_python/, plain_python_clean/     no Django
├── limits_*/                              v1 limitations, now fixed
├── v2_*/                                  data-flow model v2
├── r3_*/, r5_*/, r6_*/, r7_*/             adversarial findings (rounds 3, 5, 6, 7)
└── r8_*/                                  findings from the pr-swarm review of PR #3
```

## Test fixtures

**Bug 1** (`test_fixtures/bug1/`): `split_energy` writes to `EnergyRecord` via `objects.create()`, quantizing values to 6 decimal places. But `EnergyRecord.energy` is `DecimalField(decimal_places=3)`. The tool discovers the edges from the ORM write pattern and detects `6 > 3` from body analysis — no manual overrides needed.

**Transitive** (`test_fixtures/transitive/`): `compute_offpeak` guarantees precision ≤ 4. `split_energy` has postcondition `max(input_precision, 3)`. Pairwise, each edge is consistent. But composed: `max(4, 3) = 4 > 3` violates the model. Only graph-level checking catches this.

**Nullable** (`test_fixtures/nullable/`): `apply_discount` has a `return None` path and otherwise writes 4dp values to `Invoice.total` and `Invoice.discount`, which are `DecimalField(decimal_places=2)` with the default `null=False`. The checker reports the precision inconsistency on each write (`4 > 2`, from the docstring) and the nullability one at the `return None` site: the annotation `-> Invoice` excludes None, so it is a requirement on every `return` of `apply_discount` (and the guarantee its callers rely on). Each hop is checked with `checkEdgeAll`, which returns a result for every constraint pair, so one failing kind does not hide another.

**Plain Python** (`test_fixtures/plain_python/`): no Django. `LineItem` is a `@dataclass` and `InvoiceRecord` a pydantic `BaseModel` with `Field(max_length=32)`, `Field(decimal_places=2)` and `Field(le=100)`. Three errors are reported: `lookup_discount` returns `Optional[int]` into the non-optional `discount_pct`; `customer_label` guarantees `len(result) <= 64` into a 32-character field; and `with_tax` (4dp) flows through `normalise` (`max(input_precision, 2)`) into `total` (2dp), which only the composed path shows. `test_fixtures/plain_python_clean/` is the corrected version and passes.

Each fixture has an `expected.json` listing the errors the full pipeline should report.

## Running tests

### Benchmark harness

```bash
scripts/bench.py run [--corpus DIR ...] [--out RESULT.json] [--compare baseline.json] [--format markdown]
```

Measures whether the checker detects real bugs and catches historical issues. The harness runs the pipeline on pre-fix and post-fix versions of applications and tracks outcomes across tool versions. See `bench/README.md` for corpus formats, case definitions and outcome interpretation. Other flags: `--cli`, `--checker`, `--keep-work DIR`. Exit codes: 0 ok, 1 regression under `--compare`, 2 harness or pipeline failure. The harness's own tests run with `python3 -m unittest discover -s scripts/tests`.

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
cargo test --test e2e_v2
cargo test --test e2e_limits
cargo test --test e2e_round5
cargo test --test e2e_round6
cargo test --test e2e_review_fixes

# CLI behaviour (fake checker) and the text report (needs the Lean checker built)
cargo test --test e2e_cli
cargo test --test e2e_text_format

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

# Type-check proofs only (no executable link); Main holds the end-to-end theorems
cd prover && lake build ContractGraph ContractGraph.Main
```

`lake build` verifies the soundness theorems — per edge and per path (`checkEdge_sound`, `checkEdgeAll_sound`, `checkPath_sound(_noErrors)`), completeness of the search (`enumeratePaths_complete`, `closedStates_checkPath`) and the end-to-end `runChecker_sound_all` (exit 0 ⇒ every data path is stepwise sound) — and the test modules in `ContractGraphTest/` (BugReport1, DataflowV2, DedupeTest, NoErrorsSoundness, NullableDemo, Round3, Round5, Round6, SoundnessDemo, StateSearch, TransitiveDemo). If any proof has a gap (`sorry`), `lake build` will report a warning.

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
| pydantic | base `BaseModel`, `pydantic_settings.BaseSettings` (v1 `pydantic.BaseSettings`), `SQLModel` | nullability, `Field(max_digits, decimal_places, max_length, ge, gt, le, lt)` with int, float or Decimal bounds, `Annotated[T, Field(...)]` (also through a type alias `Percent = Annotated[...]` / `type Percent = ...`, imported or local), `con*()`, `Literal[...]`, Enum types; type only in strict mode. A field with a `field_validator(..., mode="before"/"wrap")` (v1 `validator(..., pre=True)`), or any field of a model with `model_validator(mode="before"/"wrap")` (v1 `root_validator(pre=True)`), has no requirements: the validator may transform the value first |
| `NamedTuple`, `TypedDict` | base class | type, nullability |

Classes are found at module level, inside module-level `if` / `try` / `with` blocks (django-oscar's `if not is_model_registered(...)`), and nested in class bodies (for `choices=Status.choices`). Numeric bounds are exact decimals of any size and precision (exponent notation normalised); a copy in millionths, rounded conservatively, is kept for older checkers. Django and non-strict pydantic numeric fields accept int, float and Decimal, so they carry no type contract.

**Function contracts.** Postconditions describe the return value: return annotation, analysis of the `return` expressions, docstring `ensures:`. Preconditions are per parameter: the parameter's annotation and docstring `requires:` clauses about it.

A return annotation that excludes None (`-> Invoice`, `-> str`, not `Optional[...]`, `Any`, `object` or a type variable) is a contract of the function (assume-guarantee): a target node `f.<return>` (at the annotation) requires non-null, and for `Decimal` / `str` / `bool` the type, and every `return` of `f` is an edge into it (a bare `return`, or falling off the end of a body that is not a stub, is `None`). Callers rely on the annotation, since the function's own returns are checked against it, so a body that can return None is reported once, at the return statement. Generators and overload stubs have no return contract. Value analysis understands `quantize`, `round`, `Decimal('...')` literals (including exponent notation), Decimal `+`/`-` (`max(p, q)` places) and `*` (`p + q`), `sum()` over a generator or list of Decimals (the elements' places), `min`/`max`, `await`, local variables, module and class constants (also `self.Status.PAID` on a nested class), literals, case mapping of string literals (`"ok".upper()` keeps the length and maps the value), reads of fields of a known class (the values assigned in the function, else the field's declared contracts: every write to it is checked against them), any slice (`s[a:b]` has length at most `b` when `b >= 0`, and at most `s`'s length), `max`/`min` over a generator or display (the elements' facts), exact interval arithmetic for int and Decimal `+`/`-` (`len(x) - 1 >= -1`), `Decimal(0.1)` (the float's exact binary value: 55 places), ORM results (`objects.get/create/latest` are instances, `count/update` non-negative ints, pydantic `model_validate`/`model_copy` instances), and None producers (`None`, `x if c else None`, `.get(k)`, `getattr(o, n, None)`, `next(it, None)`, `.pop(k, None)`, `re.match/search/fullmatch` and the same methods on a compiled pattern, `.first()`/`.last()`, `x and y` with a nullable operand, `min/max(..., default=None)`, calls to nullable functions, `Optional` parameters and fields). Values that cannot be None: operator results and augmented assignments, `sum/min/max/len/abs/round/int/float/str/bool/list/tuple/dict/set/sorted`, string methods, `%` formatting, `F()` expressions, enum members, generator calls (a function containing `yield`). Nullability is flow-sensitive: `if x is None: return`, `if x:`, `assert x is not None` and reassignment in a branch all narrow, and so does a dereference: after `x.attr`, `x.method(...)`, `x[i]` or `len(x)` runs (unconditionally: not the right operand of `and`/`or`, not a dunder attribute, not inside `assert`), `x` is not None for the rest of the block, since the program would have raised. `match` statements count for termination (an exhaustive `match` whose cases all return or raise does not fall through). A `None`-default parameter of a function with a project-defined decorator (anything but `staticmethod`, `classmethod`, `property`, `wraps`, `lru_cache`, `cache`, `cached_property`, `overload`, `abstractmethod`, `override`) has unknown nullability inside the function, since the decorator may supply the argument (dispatch's `@background_task` passes `db_session`): a warning where it matters, never an error. A class-body constant (`configuration = None`) whose name the project assigns on some object (`plugin.configuration = ...`) is not read as a constant.

At a write, argument or return site a value with up to 4 alternatives (the branches of `a if c else b`, the reaching definitions of a local) is one edge per alternative, all with the same site, so a branch that certainly violates a requirement is an error even when another branch is unknown; function results keep a join. A literal `None` written into a field that accepts None meets every other requirement of the field (no warning). Docstring clauses:

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
| a write whose value is the result of `g(...)`: `Cls(f=g(...))`, `x = g(...); Cls(f=x)`, positional (dataclass/attrs/NamedTuple), `**{...}` (also `**{**base, "f": v}`: the keys no later spread may replace), `Model.objects.create/update/update_or_create/get_or_create(defaults=...)` and their async forms (`acreate`, `aupdate`, `aget_or_create`, `aupdate_or_create`), `cls(...)` / `cls.objects.create(...)` in a classmethod, `obj.f = ...`, `setattr(obj, "f", v)`, `obj.__dict__.update(f=v)`, `dataclasses.replace(obj, f=v)`, `obj.model_copy(update={...})`, `Cls.model_validate({...})`, `**kw` forwarders, dicts passed to a function that splats its parameter (`def create(self, data): return Model.objects.create(**data)`), dicts bound to a local (`data = {...}`, `dict(k=v)`) or returned by a function whose every return is a dict display, splatted directly or through such a function, and attribute writes on a loop variable over a queryset (`for p in Model.objects.filter(...)`), a typed collection (`list[Model]`) or a list the function passes to `Model.objects.bulk_update(ps, ...)` | `g` (call-site node) `writes_to` `Cls.f` |
| a write whose fields are unknown on an object of a known class: `setattr(obj, name, v)` with a computed name, `Cls(**d)` / `objects.create(**d)` / `update(**d)` / `defaults=d` / `obj.__dict__.update(d)` / `model_copy(update=d)` with `d` not a known dict (also the keys after an unknown `**spread`), a forwarder's own `Cls(**kw)` when nothing in the project calls it | enclosing function `writes_to` every field of `Cls` with no guarantees (a warning where the field has a requirement) |
| `return expr` in a function whose annotation excludes None | `g` (call-site node) or the function `writes_to` `f.<return>` |
| a write of any other expression | enclosing function `writes_to` `Cls.f`, with the expression's own contracts (override) |
| `h(..., g(...), ...)` | `g` (call-site node) `flows_to` `h`, bound to the parameter |
| `h(..., expr, ...)` | enclosing function `flows_to` `h`, bound to the parameter, with `expr`'s contracts |
| `h(...)` | caller `calls` `h` (structural, not checked) |

`obj.f = v` is flow-sensitive: the values written are those of `obj.f` that reach an `obj.save()` (or `super().save()` in a model method) in the function, or its exits when it never saves `obj`, narrowed there (`if self.f is None: self.f = "x"` or `raise` before the save leaves no None). Calls are resolved through imports (`import m as a`, `from m import x`, relative imports), `self.method()`, `cls.method()`, `super().method()` (the parent's method), methods on receivers of a known class, and methods on a freshly constructed or returned object (`Cls().m()`, `make().m()`). A method call dispatches to every override in project subclasses of the receiver's class (class hierarchy analysis), `self.m()` in an inherited method included: one call-site node per target, and the call's result facts are the join over the targets; `Cls().m()` dispatches on `Cls` only. Past 32 targets the callee is unknown. Every call whose value is used gets its own call-site node, so one helper used in two places does not create impossible paths. Module-level code is analysed as a pseudo function `<module m>`. Names are short when unique and module-qualified otherwise (`billing.records.Invoice.total`).

**Checking.** Each hop of a path compares the source's guarantees with the target's requirements of the same kind, after composing dependent bounds along the path. Errors are reported once per finding with the shortest path, the failing hop and the write or call site. A hop where the target has a requirement but the source has no guarantee of that kind gives a warning, when the requirement could reject a value.

**Trust:** the Python data class semantics in `BehaviorModel.lean` are trusted-not-proved like the Django ones. For dataclass, attrs, `NamedTuple` and `TypedDict` fields the contract is the annotation, which Python does not enforce at runtime; the claim is relative to a type-correct program. pydantic enforces its constraints on construction. Return annotations are not trusted: they are checked at every `return` of the function, and callers rely on them only as that checked guarantee. Parameter annotations are the requirements of the function; a value that the extractor cannot show meets one gives a warning.

**Not covered:** a tuple return is one value (no per-element contracts); `@property` access and nested functions are not followed; values built inside comprehensions are unknown (`sum()`, `max()` and `min()` of a generator excepted); a dependent bound is only produced for single-parameter functions; a function's result is the join of its return values, so a return branch that certainly violates a requirement combined with a branch of unknown bounds only warns; `functools.partial` objects and calls on `Protocol`-typed receivers are not followed (a Protocol method's declaration stands for every implementation: its annotation, no bounds); lower bounds on string length (`min_length`) are not checked; an annotation naming a type imported from a package outside the project, other than the standard library and the modelled or widely used frameworks (Django, pydantic, attrs, SQLAlchemy, FastAPI, ...), says nothing about None (it may alias an `Optional` type, as `opentelemetry.util.types.Attributes` does), so it gives no non-null contract; a module-level dict of literals is a constant only when the project never mutates it by name (`X[k] = v`, `X.update(...)`), mutation through another reference is not seen; pydantic field aliases; DRF `ModelSerializer` writes; `max_digits` overflow of the integer part is not claimed; a method called between an attribute write and the save is assumed not to reassign the attribute; a return annotation of `float` or `int` is not a type requirement (`int` and `bool` values are acceptable there), and an annotation name that looks like a type variable (`T`, `TModel`, `T_co`) has no return contract. Writes the project cannot see are not checked: a framework calling a dict or `**kw` forwarder that the project also calls (only the project's calls are checked), raw SQL, fixtures. Checking is flow-sensitive for None but not path-sensitive, and some facts are over-approximated; each of the following can produce a false positive:

- `isinstance` guards, and conditions that make a branch infeasible (for example an exhaustive enum or status handled with a fallback `return None`, or a function that falls off the end only for values its callers never pass);
- `.get(k)` is nullable even when the key is always present (a payload built with it, a request that always carries it), and so is `.get(x)` on an object of unknown type with a `get` method (it is read as `dict.get`);
- values validated elsewhere (a manifest or schema checked before the write, a caller that always passes a value to a `None`-default parameter) are only as known as the code at the write shows;
- decorators that inject arguments are modelled only when the decorator is defined in the project; exceptions suppressed by a context manager are not modelled;
- class hierarchy analysis joins every override: a call on a base-class instance (or `self.m()` in a base-class method) is reported for a subclass's override even when that subclass never reaches the call, and a class constant read as `self.X` / `cls.X` in a base-class method takes every subclass's value (more than four values, or a computed one, is unknown);
- an attribute written on an instance in a function that never saves it is treated as written at every exit of the function (the instance may be saved later), also on exits after which the instance is discarded;
- a field read yields the field's declared contracts, so copying a field into a narrower one (`CharField(255)` into `CharField(100)`) is an error even when the stored values are shorter.

A syntax error in any file stops the run (exit 2) unless `--allow-parse-errors` is given.

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
| 0 | Every data path (function → model node) consistent |
| 1 | One or more inconsistencies found |
| 2 | Extraction, parse or translation failure (including an unreadable database or a malformed contract row), or an incomplete check (a `--max-states` / `--max-states-per-edge` budget exceeded — nothing is verified — or the checker crashed or produced no JSON) |

`--max-states N` (alias `--max-paths N`) and `--max-states-per-edge N` are passed to the checker after the database path.
