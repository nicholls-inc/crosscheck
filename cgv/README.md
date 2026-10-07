# Contract Graph Verifier

A three-layer pipeline that extracts implicit contracts from Python application code (Django models, dataclasses, attrs, pydantic, `NamedTuple`, `TypedDict`), translates them into Lean propositions, and checks their consistency across component boundaries with machine-checked soundness guarantees.

**Status:** research prototype. It began as a three-node PoC (function A → function B → data model field) demonstrating that graph-level consistency checking catches bugs that pairwise checking structurally misses, and now checks real-scale graphs with a state-based checker. The target can be a Django model field or a field of a plain-Python data class; see [Python support](#python-support).

## When to use it

Use CGV next to mypy or pyright, not instead of them. A type checker asks whether a value has the right type. CGV asks whether a value meets the constraints that a model field or a data class declares: length (`max_length`), precision (`decimal_places`), range (validators, `ge`, `le`, and the integer digits that `max_digits` allows), choices, and nullability. It checks these across several hops, from the function that produces a value through the functions that pass it on to the field that stores it.

Tests often miss these bugs too, because the database does not reject the value. PostgreSQL rounds a `numeric` value with more decimal places than the column's scale ([PostgreSQL 16 docs, Numeric Types](https://www.postgresql.org/docs/16/datatype-numeric.html#DATATYPE-NUMERIC-DECIMAL)), and SQLite stores a string of any length in a `VARCHAR(10)` column ([SQLite FAQ, question 9](https://www.sqlite.org/faq.html#q9)).

Type checkers reach bugs that CGV does not. CGV reports a value only where it reaches a declared constraint, so a `None` dereference with no constrained target is not yet reached. The blocking property is that there is no declared constraint to check the value against, and the open question is which unconstrained dereferences a nullability model should treat as targets. A pydantic model built from `response.json()`, where the remote API may send null (`external-payload-null`), is also not yet reached. The blocking property is that nothing in project code produces the `None`, so no function node in the graph is the source of the bad value, and the open question is whether a call that parses an external payload should be a source node with an unknown value. Some nullability paths reach CGV as warnings, not errors (see [What exit 0 promises](#what-exit-0-promises)).

**Comparison on the bench corpus.** `scripts/typecheck_compare.py` runs mypy and pyright on the pre-fix and fixed trees of each case in `bench/corpus`. A type checker flags a case when the pre-fix tree has an error in the bug's file, of a rule that the fix removes. The CGV column is the outcome in `bench/baseline.json` (see `bench/README.md`). Rerun the table with Python 3.12 or later:

```bash
python3 -m venv /tmp/tc && /tmp/tc/bin/pip install -r bench/typecheckers/requirements.txt
scripts/typecheck_compare.py --python /tmp/tc/bin/python
```

mypy 2.4.0 (`strict`, django-stubs 6.1.2, pydantic plugin), pyright 1.1.414 (`strict`), Django 6.1.2, pydantic 2.13.5. CGV outcome from `bench/baseline.json`.

| Case | Kind | CGV design reaches it | CGV | mypy | pyright |
|---|---|---|---|---|---|
| `dict-get-default` | non-null | yes | WARNING_ONLY | not flagged | not flagged |
| `external-payload-null` | non-null | not yet reached | NOT_REPORTED | not flagged | not flagged |
| `first-under-type-ignore` | non-null | yes | CAUGHT | not flagged | not flagged |
| `guard-in-caller` | non-null | yes | DETECTED_NOT_CLEARED | not flagged | not flagged |
| `length-address-defaults` | length | yes | CAUGHT | not flagged | not flagged |
| `length-cross-model` | length | yes | CAUGHT | not flagged | not flagged |
| `none-dereference` | non-null | not yet reached | NOT_REPORTED | flagged: line 6 `union-attr` | flagged: line 6 `reportOptionalMemberAccess` |
| `optional-fields-to-params` | non-null | yes | CAUGHT | not flagged | not flagged |
| `precision-two-hop` | precision | yes | CAUGHT | not flagged | not flagged |
| `two-attribute-deep` | non-null | yes | WARNING_ONLY | flagged: line 5 `misc` | not flagged |

Read the table with three facts in mind. The corpus has 10 synthetic cases, written to exercise CGV, so the table shows which tool reaches which kind of bug, not how often each kind occurs. Three cases (`first-under-type-ignore`, `guard-in-caller`, `optional-fields-to-params`) silence the type checker with `# type: ignore`, as real code does, and CGV does not read those comments. pyright with django-stubs reports the type of each Django model field read as unknown (`Type of "summary" is unknown`), so it cannot see that a field is nullable. django-stubs resolves field types through its mypy plugin, which pyright does not load. The line in a `flagged` cell is the line of the type checker's error, which need not be the line of the bug.

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
| Translation | Not proved — no theorems yet. `translateRows` translates every row or rejects the database (exit 2), so no row is dropped. It rejects an unknown `constraint_type`, `verification_level`, `contract_role` or `relationship`, a node kind other than `model`, `function` or `field`, two rows with one id, an edge or contract row that names a missing node, a per-edge contract row whose edge is missing, has no `source_override`, starts at another node or is not a postcondition, a row with neither the value its kind reads nor a `dependent_expr`, and a malformed `dependent_expr`, `param_choices` or decimal bound. The SQL reads that produce the rows (`readNodes`, `readContracts`, `readEdges`) are not tested yet; TB-1.21 tests them |
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
| Django model | base `models.Model`, or a project model (abstract or concrete, across modules) | `DecimalField` precision and `max_digits` (as a range), `CharField` length, `null`, `choices` (literal, constant, `TextChoices`/`IntegerChoices`, also nested in the model), `MinValueValidator`/`MaxValueValidator`, `Positive*Field`, implicit defaults |
| dataclass | `@dataclass`, `@dataclasses.dataclass(...)` | type, nullability |
| attrs | `@attr.s`, `@attr.define`, `@attrs.define`, `@define`, `@frozen`, `@mutable` | type, nullability |
| pydantic | base `BaseModel`, `pydantic_settings.BaseSettings` (v1 `pydantic.BaseSettings`), `SQLModel` | nullability, `Field(max_digits, decimal_places, max_length, ge, gt, le, lt)` with int, float or Decimal bounds, `Annotated[T, Field(...)]` (also through a type alias `Percent = Annotated[...]` / `type Percent = ...`, imported or local), `con*()`, `Literal[...]`, Enum types; type only in strict mode. A field with a `field_validator(..., mode="before"/"wrap")` (v1 `validator(..., pre=True)`), or any field of a model with `model_validator(mode="before"/"wrap")` (v1 `root_validator(pre=True)`), has no requirements: the validator may transform the value first |
| `NamedTuple`, `TypedDict` | base class | type, nullability |

`max_digits=m` with `decimal_places=d` becomes a range requirement of `±(10^(m-d) - 10^-d)` (`±999.99` for 5 and 2), the largest magnitude with at most `m - d` integer digits and `d` fractional digits, intersected with any validator bounds; without a `decimal_places` the extractor can read (pydantic's `Field(max_digits=4)`, or a Django `decimal_places` given by a variable) it is `±(10^m - 1)`. With a readable `decimal_places`, together with the precision requirement this is the integer-digit and total-digit check of Django's `DecimalValidator` and of pydantic, apart from two classes that are not yet reached. Zero written as `Decimal("0")` or `Decimal(0)` into a field with `max_digits == decimal_places` is accepted although Django and pydantic reject it (they count the zero as one whole digit); the blocking property is that a range bound cannot exclude one interior value, and the open question is whether that quirk is worth a check of its own. A `max_digits` above 38 or a `decimal_places` above 30 gives no range requirement; the blocking property is the size of `Dec` (an `i128` mantissa, scale at most 30), and the open question is whether any real schema needs a wider bound. A `max_digits` without a readable `decimal_places` also limits the total digits of a value with a fractional part (`99998.5` has 6 digits), which the range does not check: that conjunct is not yet reached, the blocking property is that the extractor has no total-digit guarantee for a source, and the open question is whether such a guarantee is worth deriving. A write of a value whose magnitude the extractor cannot bound gets the usual range warning. A read of the field carries the range as a guarantee, as a `CharField` read carries its `max_length`, so copying a wider decimal field into a narrower one is an error. `scripts/max_digits_oracle.py` checks the range against Django's `DecimalValidator` and pydantic.

Classes are found at module level, inside module-level `if` / `try` / `with` blocks (django-oscar's `if not is_model_registered(...)`), and nested in class bodies (for `choices=Status.choices`). Numeric bounds are exact decimals of any size and precision (exponent notation normalised); a copy in millionths, rounded conservatively, is kept for older checkers. Django and non-strict pydantic numeric fields accept int, float and Decimal, so they carry no type contract.

A type contract names the type an annotation refers to, resolved in the annotating module. The names `int`, `float`, `str`, `bool`, `Decimal` and pydantic's `StrictInt`, `StrictFloat`, `StrictStr` and `StrictBool` mean the builtin, `decimal.Decimal` or the pydantic type only when the module leaves them unbound or imports them from there. When the module binds one to a project class (`from units import float`, or `class int:` in the module), the contract names the class by its qualified name (`units.float`), so a `units.float` written into a `float` field is an error. When the module binds one to anything else (a function, a constant, a rebound global, a type from another package), the annotation gives no type contract. This applies to parameter, return and data class field annotations, through `Optional`, unions, `Annotated`, `Final`, quoted forward references and an alias defined in the same module that is the whole annotation, as an unquoted name (an alias nested in another annotation, defined through another alias, or quoted drops the type contract). An annotation that names several types (including a generic such as `list[str]`, but not `None`), one of them a project class that shadows a type-contract name, gives no type contract, and so does the element type of a `tuple[...]` return annotation whose elements do not all resolve alike. Not yet reached: a union or a mixed tuple that names a project class, where a wrong-typed write into the project class member is not reported. The blocking property is that an annotation carries one type name, not one per member, and the open question is whether each member should carry its own contract (CG-1.50). Not yet reached: a name bound in a class body or a function, a name a star import from outside the project may bind, a name imported from a project module that re-exports the real type (the type contract is dropped, so such a write is no longer reported), an annotation alias defined in another module (it is read in the importing module's bindings, not the defining module's), and the pydantic `con*()` helpers. The blocking property is that the resolver reads only module-level definitions and imports, and the open question is whether it should report a name it cannot resolve so that the type contract is dropped (CG-1.50). A project class that subclasses the builtin (`class float(builtins.float)`) is reported when it reaches a `float` requirement, a false positive, because type contracts compare names and not class hierarchies.

**Function contracts.** Postconditions describe the return value: return annotation, analysis of the `return` expressions, docstring `ensures:`. Preconditions are per parameter: the parameter's annotation and docstring `requires:` clauses about it.

A return annotation that excludes None (`-> Invoice`, `-> str`, not `Optional[...]`, `Any`, `object` or a type variable) is a contract of the function (assume-guarantee): a target node `f.<return>` (at the annotation) requires non-null, and for `Decimal` / `str` / `bool` the type, and every `return` of `f` is an edge into it (a bare `return`, or falling off the end of a body that is not a stub, is `None`). Callers rely on the annotation, since the function's own returns are checked against it, so a body that can return None is reported once, at the return statement. Generators and overload stubs have no return contract. Value analysis understands `quantize`, `round`, `Decimal('...')` literals (including exponent notation), Decimal `+`/`-` (`max(p, q)` places) and `*` (`p + q`), `sum()` over a generator or list of Decimals (the elements' places), `min`/`max`, `await`, local variables, module and class constants (also `self.Status.PAID` on a nested class), literals, case mapping of string literals (`"ok".upper()` keeps the length and maps the value), reads of fields of a known class (the values assigned in the function, else the field's declared contracts: every write to it is checked against them), any slice (`s[a:b]` has length at most `b` when `b >= 0`, and at most `s`'s length), `max`/`min` over a generator or display (the elements' facts), exact interval arithmetic for int and Decimal `+`/`-` (`len(x) - 1 >= -1`), `Decimal(0.1)` (the float's exact binary value: 55 places), ORM results (`objects.get/create/latest` are instances, `count/update` non-negative ints, pydantic `model_validate`/`model_copy` instances), and None producers (`None`, `x if c else None`, `.get(k)`, `getattr(o, n, None)`, `next(it, None)`, `.pop(k, None)`, `re.match/search/fullmatch` and the same methods on a compiled pattern, `.first()`/`.last()`, `x and y` with a nullable operand, `min/max(..., default=None)`, calls to nullable functions, `Optional` parameters and fields). Values that cannot be None: operator results and augmented assignments, `sum/min/max/len/abs/round/int/float/str/bool/list/tuple/dict/set/sorted`, string methods, `%` formatting, `F()` expressions, enum members, generator calls (a function containing `yield`). Nullability is flow-sensitive: `if x is None: return`, `if x:`, `assert x is not None`, `if x in {"a", "b"}:` (a display of non-None literals), reassignment in a branch and reassignment in a `try` whose handlers exit or rebind all narrow, and so does a dereference: after `x.attr`, `x.method(...)`, `x[i]` or `len(x)` runs (unconditionally: not the right operand of `and`/`or`, not a dunder attribute, not inside `assert`), `x` is not None for the rest of the block, since the program would have raised. After `k in d`, where `d` is known to be a dict (a parameter annotated `dict`, `Dict`, `Mapping`, `MutableMapping`, `defaultdict` or `OrderedDict` that the body does not rebind, or a local built only by a dict display, a dict comprehension or `dict()` / `defaultdict()` / `OrderedDict()`), `d.get(k)` has the facts of `d[k]` (unknown nullability, so a warning rather than an error) until `d` or `k` is rebound, or a call other than `d.get`, a `del`, a walrus or a suspension (`await`, `yield`, `async for`, `async with`, an async comprehension) runs (also across a `try` body that runs one, a `match` statement that has one, a loop that has one, an earlier operand of `and` / `or`, the test of a conditional expression, and any part of a comprehension; a `while` test is read after the loop's own effects, since it runs again after every iteration). A comprehension filter gives the element no fact. A field read through a parameter `p.f` has unknown nullability when every call of the function narrows `arg.f`, and the project shows every caller: at least one call from outside the function and from outside any cycle of calls that nothing else enters, every call by the function's name resolves to it, no other use of the name, no decorator, not async and not a generator (its body runs after the call, when the caller may have cleared the field), not a dunder method, and for a method only project bases. A call that passes `*args` or `**kwargs` counts as unguarded. No function qualifies when the project calls `getattr`, `vars(o)`, `operator.attrgetter` or `operator.methodcaller` with a computed name, and none in a module that calls `globals()`, `locals()` or `vars()` or is imported by name into one; a string literal there is a use of that name. The guard holds only until the callee may hand `p` to other code: a call with `p` as an argument, a method call on `p`, any other read of `p` as a value (`q = p`, a closure, an operand), or a suspension. This lowers an error to a warning, since a caller outside the project may still pass None. `match` statements count for termination (an exhaustive `match` whose cases all return or raise does not fall through). A `None`-default parameter of a function with a project-defined decorator (anything but `staticmethod`, `classmethod`, `property`, `wraps`, `lru_cache`, `cache`, `cached_property`, `overload`, `abstractmethod`, `override`) has unknown nullability inside the function, since the decorator may supply the argument (dispatch's `@background_task` passes `db_session`): a warning where it matters, never an error. A class-body constant (`configuration = None`) whose name the project assigns on some object (`plugin.configuration = ...`) is not read as a constant.

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
| a write whose value is the result of `g(...)`: `Cls(f=g(...))`, `x = g(...); Cls(f=x)`, positional (dataclass/attrs/NamedTuple), `**{...}` (also `**{**base, "f": v}`: the keys no later spread may replace), `Model.objects.create/update/update_or_create/get_or_create(defaults=...)` and their async forms (`acreate`, `aupdate`, `aget_or_create`, `aupdate_or_create`), `cls(...)` / `cls.objects.create(...)` in a classmethod, `obj.f = ...`, `setattr(obj, "f", v)`, `obj.__dict__.update(f=v)`, `dataclasses.replace(obj, f=v)`, `obj.model_copy(update={...})`, `Cls.model_validate({...})` into a field whose contract validation does not enforce (see **Trust**), `**kw` forwarders, dicts passed to a function that splats its parameter (`def create(self, data): return Model.objects.create(**data)`), dicts bound to a local (`data = {...}`, `dict(k=v)`) or returned by a function whose every return is a dict display, splatted directly or through such a function, and attribute writes on a loop variable over a queryset (`for p in Model.objects.filter(...)`), a typed collection (`list[Model]`) or a list the function passes to `Model.objects.bulk_update(ps, ...)` | `g` (call-site node) `writes_to` `Cls.f` |
| a write whose fields are unknown on an object of a known class: `setattr(obj, name, v)` with a computed name, `Cls(**d)` / `objects.create(**d)` / `update(**d)` / `defaults=d` / `obj.__dict__.update(d)` / `model_copy(update=d)` with `d` not a known dict (also the keys after an unknown `**spread`), a forwarder's own `Cls(**kw)` when nothing in the project calls it | enclosing function `writes_to` every field of `Cls` with no guarantees (a warning where the field has a requirement) |
| `return expr` in a function whose annotation excludes None | `g` (call-site node) or the function `writes_to` `f.<return>` |
| a write of any other expression | enclosing function `writes_to` `Cls.f`, with the expression's own contracts (override) |
| `h(..., g(...), ...)` | `g` (call-site node) `flows_to` `h`, bound to the parameter |
| `h(..., expr, ...)` | enclosing function `flows_to` `h`, bound to the parameter, with `expr`'s contracts |
| `h(...)` | caller `calls` `h` (structural, not checked) |

`obj.f = v` is flow-sensitive: the values written are those of `obj.f` that reach an `obj.save()` (or `super().save()` in a model method) in the function, or its exits when it never saves `obj`, narrowed there (`if self.f is None: self.f = "x"` or `raise` before the save leaves no None). Calls are resolved through imports (`import m as a`, `from m import x`, relative imports), `self.method()`, `cls.method()`, `super().method()` (the parent's method), methods on receivers of a known class, and methods on a freshly constructed or returned object (`Cls().m()`, `make().m()`). A method call dispatches to every override in project subclasses of the receiver's class (class hierarchy analysis), `self.m()` in an inherited method included: one call-site node per target, and the call's result facts are the join over the targets; `Cls().m()` dispatches on `Cls` only. Past 32 targets the callee is unknown. Every call whose value is used gets its own call-site node, so one helper used in two places does not create impossible paths. Module-level code is analysed as a pseudo function `<module m>`. Names are short when unique and module-qualified otherwise (`billing.records.Invoice.total`).

**Checking.** Each hop of a path compares the source's guarantees with the target's requirements of the same kind, after composing dependent bounds along the path. Errors are reported once per finding with the shortest path, the failing hop and the write or call site. A hop where the target has a requirement but the source has no guarantee of that kind gives a warning, when the requirement could reject a value.

**Trust:** the Python data class semantics in `BehaviorModel.lean` are trusted-not-proved like the Django ones. For dataclass, attrs, `NamedTuple` and `TypedDict` fields the contract is the annotation, which Python does not enforce at runtime; the claim is relative to a type-correct program. pydantic enforces its constraints on construction. `Cls.model_validate(...)`, `model_validate_json`, `model_validate_strings` and the v1 `parse_obj` and `parse_raw` take untyped input and reject an invalid value with a `ValidationError`, so they are a validation boundary: an entry for a field whose contract validation enforces is not a write. An entry stays a write for a field with `decimal_places` or `max_digits` (pydantic counts digits after it drops trailing zeros, so it stores `Decimal("1.2300")` in a `decimal_places=2` field), for `SkipValidation[T]` or a `PlainValidator` or `WrapValidator` in `Annotated` metadata (each can store a value validation would reject), and for a `None` default under an annotation without None (pydantic v1 reads it as Optional). An entry is not a write only for a field whose annotation names nothing but types and `Annotated` metadata that CGV knows pydantic validates (the builtin scalars and containers, `Decimal`, the date and time types, `UUID`, `Optional`, `Union`, `Literal`, the `Strict*`, constrained and URL types, `Field`, the `annotated_types` constraints and `BeforeValidator`); a renamed import, an alias, a project class, a validator held in a variable and an `AfterValidator` (whose result is not revalidated) all keep the entry as a write, as on a constructor call. Not yet reached: the arguments of a generic container (`list[SkipValidation[str]]` reads as a validated `list`) and an aliased `field_validator` or `model_validator` decorator; the blocking property is that `walk_annotation` reads only the head name of a container and `transforming_validators` matches a decorator by the last segment of its name, and the open question is whether to walk the container arguments and resolve decorator names (CG-1.44). A project name that shadows one of those known names (a local class `Decimal`) is not yet reached: the extractor matches the last segment of a name and does not resolve what it imports, and the open question is whether to resolve imports first, which would also let a nested model's entries stop being writes (CG-1.43). A typed constructor call `Cls(f=v)` is a write for every field. Return annotations are not trusted: they are checked at every `return` of the function, and callers rely on them only as that checked guarantee, with these exceptions. Three assumptions are trusted unchecked, because they decide where a body ends: a project function annotated `NoReturn` or `Never` never returns (it has no return contract, so CGV does not check its `return`s), a parameter annotated with a project enum holds one of its members (a `match` that names every member ends the flow), and `sys.exit`, `os._exit`, `os.abort`, `typing.assert_never`, `typing_extensions.assert_never` and the builtins `exit` and `quit` never return. A call inside a `with` body is not yet reached: it is not treated as an exit, because a context manager may suppress what it raises; the blocking property is that the extractor does not know which context managers propagate exceptions, and the open question is whether to recognise a closed list of them (CG-1.47). The same holds for a `raise` that ends a `with` body, and what a `with` body narrows does not survive it. An exception raised before the `return` that ends a `with` body, by its value, or by the expression of a later `with` item, under a manager that suppresses it is not yet reached, for the same reason (CG-1.47). A violated assumption can hide a non-null error, so the claim holds for a type-correct program, as for dataclass fields. Not yet reached: a method that a subclass or a class-body assignment overrides, a nested function that rebinds the subject through `nonlocal`, and a star import that rebinds `sys`; each can make a call or a `match` look like an exit when it is not. The blocking property is that the resolver searches readable base classes depth-first and skips unreadable ones, `rebound` does not record an assignment through a class name, `bound_names` does not look inside nested functions, and the resolver cannot tell what a star import exports; the open question is whether the resolver should report an uncertain resolution so that callers can refuse it (CG-1.21, CG-1.22, CG-1.25 and CG-1.27). Parameter annotations are the requirements of the function; a value that the extractor cannot show meets one gives a warning.

**Not covered:** a tuple return is one value (no per-element contracts); `@property` access and nested functions are not followed; values built inside comprehensions are unknown (`sum()`, `max()` and `min()` of a generator excepted); a dependent bound is only produced for single-parameter functions; a function's result is the join of its return values, so a return branch that certainly violates a requirement combined with a branch of unknown bounds only warns; `functools.partial` objects and calls on `Protocol`-typed receivers are not followed (a Protocol method's declaration stands for every implementation: its annotation, no bounds); lower bounds on string length (`min_length`) are not checked; an annotation naming a type imported from a package outside the project, other than the standard library and the modelled or widely used frameworks (Django, pydantic, attrs, SQLAlchemy, FastAPI, ...), says nothing about None (it may alias an `Optional` type, as `opentelemetry.util.types.Attributes` does), so it gives no non-null contract; a module-level dict of literals is a constant only when the project never mutates it by name (`X[k] = v`, `X.update(...)`), mutation through another reference is not seen; pydantic field aliases; DRF `ModelSerializer` writes; a method called between an attribute write and the save is assumed not to reassign the attribute; a return annotation of `float` or `int` is not a type requirement (`int` and `bool` values are acceptable there), and an annotation name that looks like a type variable (`T`, `TModel`, `T_co`) has no return contract. Writes the project cannot see are not checked: a framework calling a dict or `**kw` forwarder that the project also calls (only the project's calls are checked), raw SQL, fixtures. Checking is flow-sensitive for None but not path-sensitive, and some facts are over-approximated; each of the following can produce a false positive:

- `isinstance` guards, and conditions that make a branch infeasible (for example an exhaustive `match` over a member list the extractor cannot read, a method called through an instance that never returns, or a function that falls off the end only for values its callers never pass; a call of a `NoReturn` function, `sys.exit` or a `match` over every member of a project enum is understood: see Trust above);
- `.get(k)` is nullable even when the key is always present (a payload built with it, a request that always carries it), and so is `.get(x)` on an object of unknown type with a `get` method (it is read as `dict.get`);
- values validated elsewhere (a manifest or schema checked before the write, a caller that always passes a value to a `None`-default parameter) are only as known as the code at the write shows;
- decorators that inject arguments are modelled only when the decorator is defined in the project; every context manager is assumed to suppress what its body raises, so what a `with` body narrows does not survive it and a `raise` that ends it does not end the flow, even for a lock or a file (CG-1.47);
- class hierarchy analysis joins every override: a call on a base-class instance (or `self.m()` in a base-class method) is reported for a subclass's override even when that subclass never reaches the call, and a class constant read as `self.X` / `cls.X` in a base-class method takes every subclass's value (more than four values, or a computed one, is unknown);
- an attribute written on an instance in a function that never saves it is treated as written at every exit of the function (the instance may be saved later), also on exits after which the instance is discarded;
- a field read yields the field's declared contracts, so copying a field into a narrower one (`CharField(255)` into `CharField(100)`) is an error even when the stored values are shorter. The one exception is nullability under caller guards (see value analysis): when every caller narrows the field, the read has unknown nullability, and the error becomes a warning.

A syntax error in any file stops the run (exit 2) unless `--allow-parse-errors` is given.

**Excluding files.** `--exclude GLOB` (repeatable) skips every file whose path relative to the application root, or one of whose directories, matches the glob (`*` and `?` within a path segment, `**` across segments; no leading `**` means anchored at the root). For Django projects whose tests build invalid unsaved instances on purpose, use `--exclude '**/tests/**'` (and `--exclude '**/test_*.py'` for test modules outside a `tests` directory).

## Output

`contracts check` writes JSON to stdout by default (the Lean checker's own output, passed through unchanged). Pass `--format text` for a human-readable report instead. Both formats exit with the checker's semantic exit code, except in [baseline mode](#baseline-mode).

The text report prints each error and each warning as a block. A missing guarantee, where a hop's source has no guarantee of the constraint kind that its target requires, is not a finding, so by default the report does not print it. It counts it as an unverified requirement in the `COVERAGE BY MODULE` section, which gives, for each top-level module, the edges checked and the unverified requirements by kind, and in the `RESULT:` line. `--warnings` also prints each unverified requirement as an `UNVERIFIED` block ("no nullability guarantee for the value `f` passes to `T`"). `--no-warnings` hides the warning blocks as well. Every hidden block is still counted. An incomplete run (exit 2) has no coverage section. The JSON output keeps every warning unchanged. The rules are UC-1 to UC-10 in `intent/2026-10-07-cgv-unverified-coverage-spec.md`.

```
$ ./target/release/crosscheck-contracts contracts check test_fixtures/transitive/ \
    --lean-checker ./prover/.lake/build/bin/contract-graph-checker --format text
CONTRACTS CHECKED: 18
EDGES CHECKED: 7
STATES CHECKED: 8

ERROR  utils.py:6 → models.py:5
       compute_offpeak guarantees precision ≤ 4
       EnergyRecord.energy requires precision ≤ 3
       Path: compute_offpeak → split_energy → EnergyRecord.energy
       Failing hop: split_energy → EnergyRecord.energy
       At: utils.py:27
       Path verification level: ASSUMED
       Note: invisible to pairwise checking.
       Suggestion: Source guarantees ≤ 4, target requires ≤ 3. Either tighten the source or widen the target.

WARNING  utils.py:16 → models.py:5
       split_energy guarantees precision (dependent)
       EnergyRecord.energy requires precision ≤ 3
       Path: split_energy → EnergyRecord.energy
       At: utils.py:27
       Path verification level: EXTRACTED
       Suggestion: Dependent expression on split_energy could not be resolved. Check that upstream postconditions provide the required input bindings.

COVERAGE BY MODULE
  utils: 7 edges checked, 0 requirements unverified

RESULT: 1 error, 1 warning, 0 unverified. Exit code 1.
```

On `test_fixtures/r6_alternatives/` the coverage section reads:

```
COVERAGE BY MODULE
  bug: 12 edges checked, 2 requirements unverified (precision 1, range_min 1)
  ok: 12 edges checked, 0 requirements unverified
An unverified requirement is not yet reached: the value's source has no guarantee of the kind the target requires, so the requirement passes vacuously. Open question: which guarantee the extractor could infer for such a value, or which annotation it should ask for. Pass --warnings to list them.

RESULT: 3 errors, 0 warnings, 2 unverified. Exit code 1.
```

### Baseline mode

A whole-project run reports every finding in the project. On a pull request, baseline mode reports only the findings that the change introduces.

- `--write-baseline PATH` writes the findings of the run to a baseline file. The run's output and exit code do not change, except that a site file that cannot be read makes the run print an error and exit 2 before any report. A run that is incomplete or does not produce a checker result writes no file, and removes any file already at PATH.
- `--baseline PATH` compares the run with a baseline file. Only the findings that are not in the baseline are printed. The ones that are in it are counted on a `BASELINE:` line (text) or under `baseline.existing` (JSON). Baseline findings that the run no longer reports are listed as fixed. The run exits 1 when any error is not in the baseline, 0 when every error is, and 2 when it is incomplete or the baseline file is missing or malformed.
- `--baseline` and `--write-baseline` cannot name the same file, through any spelling of the path or a symbolic link. The run exits 2 before the checker runs, because overwriting the baseline with the run's new errors would let the next run exit 0. Write the new baseline to another path.

A finding matches a baseline entry when its class, source, target, failing hop, guarantee, requirement, site file and the text of the site's line are equal. The key holds no line number, so an edit above a finding does not make it new. When the run has more findings with one key than the baseline has, every finding with that key is printed as new. A change that removes one finding and adds an equal one, with the same key, is matched and not reported. That case is not yet reached: the blocking property is that the key holds no position, and the open question is whether a finer field can tell such a pair apart without making unrelated edits churn the key. The rules are BL-1 to BL-11 in `intent/2026-10-07-cgv-baseline-mode-spec.md`.

In CI, make the baseline from the pull request's base commit and compare the head with it:

```bash
git worktree add ../base "$BASE_SHA"
crosscheck-contracts contracts check ../base/app --write-baseline baseline.json \
  --lean-checker "$CHECKER" > /dev/null || true
crosscheck-contracts contracts check app --baseline baseline.json \
  --lean-checker "$CHECKER" --format text
```

Pass the same options to both runs. Different options, or another CGV version, can only make more findings new. The first run's exit code is ignored on purpose: a base with findings still gives a baseline. If the first run is incomplete it writes no baseline, and the second run exits 2.

### Evidence record

`--evidence-record PATH` writes a JSON evidence record to PATH when the run exits 0. It holds one claim, `cgv-data-paths`, with strength `proved`, the theorem `ContractGraph.runChecker_sound_all`, the trusted base with pinned versions, and a command that reruns the check from the root of the project's git work tree. Any other outcome of a run that starts removes the file at PATH and writes none; an invocation the argument parser rejects leaves it untouched.

The checked path and the `--overrides` file must sit in a git work tree with no changes under them. A dirty or non-git checkout is refused with exit 2 before extraction, and the checker does not run. A git-ignored `.py` file that the run would analyse is refused the same way, because the commit does not pin it; remove it or pass an `--exclude` glob that skips it. A git-ignored `--overrides` file is refused too. Other ignored files do not matter. Content that git does not pin, such as a symlink target outside the work tree, is not yet reached; see the spec's concerns. After the checker exits 0, CGV checks the checkout and HEAD again just before writing the record, and exits 2 without a record if either changed during the run. If a run is killed between creating the record's temporary file and renaming it, a leftover `.NAME.tmp-PID` file beside the record can make the next run refuse with a dirty checkout when the record sits inside the checked path; delete it by hand. The format is in `intent/2026-10-06-evidence-record-spec.md` (rules EV-1 to EV-12). What CGV writes is in `intent/2026-10-06-cgv-evidence-record-spec.md` (requirements CR-1 to CR-8).

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Every data path (function → model node) that the extractor discovered is consistent; see [What exit 0 promises](#what-exit-0-promises) |
| 1 | One or more inconsistencies found |
| 2 | Extraction, parse or translation failure (including an unreadable database or a row that translation rejects, see [Trust model](#trust-model)), or an incomplete check (a `--max-states` / `--max-states-per-edge` budget exceeded — nothing is verified — or the checker crashed or produced no JSON) |

`--max-states N` (alias `--max-paths N`) and `--max-states-per-edge N` are passed to the checker after the database path.

With `--baseline`, exit 0 means that every error the checker reported was in the baseline, and exit 1 that some error was not (see [Baseline mode](#baseline-mode)). It does not carry the promise below, so `--baseline` cannot be combined with `--evidence-record`.

### What exit 0 promises

`runChecker_sound_all` proves that exit 0 means every data path of the translated graph is stepwise sound. These qualifiers apply (see [Trust model](#trust-model) for what is trusted and not proved):

- **Only the paths the extractor discovered.** Extraction is not proved (see [Trust model](#trust-model)). A write the extractor does not see is not in the graph, so exit 0 says nothing about it.
- **Writes outside the project's code are not yet reached.** This covers writes that Django or DRF makes on the project's behalf (the admin, `ModelForm.save()`, a DRF serializer's `save()`), raw SQL, and fixtures. The property that blocks them is that the write happens in framework code or in the database, which the extractor does not read. The open question is how small a model of each framework's write paths can be and still be trusted.
- **Only paths that end at a model node.** A data path runs from a function node to a model node. A `flows_to` edge into a callee that never reaches a model node is not yet reached. The blocking property is that the theorem's notion of a data path (`IsDataPath`) requires the path to end at a model node, so it says nothing about a hop that has no route to one. The open question is how a callee's contract should be modelled when the callee has no model node.
- **Relative to the translated constraints, not to Django or pydantic.** Translation from the database to Lean is not proved, and `BehaviorModel.lean` is trusted, not proved. No theorem links the checked constraints to it. For dataclass, attrs, `NamedTuple` and `TypedDict` fields the contract is an annotation that Python does not enforce, so the claim holds for a type-correct program. A program that is not type-correct is not yet reached: the blocking property is that CGV does not read the type checker's verdict, and the open question is whether it should require one.
- **A warning can rest on the project's callers.** A read that every caller guards, or that follows `k in d`, has unknown nullability, so CGV warns where it used to report an error, and exit 0 no longer says such a read is non-None. A dict can store None under a key, so `d.get(k)` after `k in d` may still be None. The caller rule assumes the project shows every caller. A computed `getattr`, `globals()` and `locals()` switch it off, but a call through a registry of functions keyed by string, or through framework code, is not yet reached. The blocking property is that the extractor builds no call graph through string-keyed dispatch or framework code, and the open question is which registries and frameworks need a model. A callee that changes `p.f` through an alias it does not pass (a global that already holds `p`, a back-reference such as `p.child.parent`, a property getter with side effects) is not yet reached either. The blocking property is that the extractor has no heap or global alias analysis, and the open question is whether a summary of the globals and fields each function writes is cheap enough to close it. Within one function, `if p.f:` still survives a call such as `clear(p)`: that attribute narrowing predates caller guards and is not yet reached for the same reason.
- **A missing guarantee is a warning, not an error.** When a hop's target has a requirement that could reject a value and the source has no guarantee of that kind, CGV warns and still exits 0. The theorem's "stepwise sound" (`stepwiseSound`) is vacuously true for such a hop: it compares only guarantees and requirements of the same kind, so it proves nothing about the requirement that has no guarantee. That is why `runChecker_sound_all` can hold for runs that exit 0 with warnings. That every such case produces a warning is the checker's behaviour and is not proved. A warning names a requirement that CGV could not show, so read a run that exits 0 with warnings as having unknown paths, not consistent ones. The text report does not print these warnings by default, but counts them as unverified requirements in its coverage section and its `RESULT:` line (see [Output](#output)).
