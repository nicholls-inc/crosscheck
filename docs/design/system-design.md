# Contract Graph Verifier PoC: System Design

**Date:** 2026-03-25
**Status:** Draft
**Grounding:** User stories (`contract-graph-poc-user-stories.md`), foundational research (`Contract_graph_verification_-_foundational_research_.md`), discussion summary (`discussion-summary.md`)

---

## 1. System overview

The contract graph verifier is a three-layer pipeline that extracts implicit contracts from Django application code, translates them into Lean propositions, and checks their consistency across component boundaries with machine-checked soundness guarantees.

The PoC targets a three-node graph: pure function A → pure function B → Django model. The goal is to demonstrate that graph-level consistency checking catches bugs that pairwise checking structurally misses, grounded in the field report's precision mismatch bug (function guarantees precision ≤ 6, Django model requires precision ≤ 3).

```
┌─────────────────────────────────────────────────────────┐
│                    Developer's Django Project            │
│  models.py, utils.py, type annotations, docstrings      │
└──────────────────────┬──────────────────────────────────┘
                       │
           ┌───────────▼───────────┐
           │   Layer 1 (Rust)      │
           │   AST Extraction      │
           │   + Edge Discovery    │
           │   + Body Analysis     │
           │   + Defaults Table    │
           └───────────┬───────────┘
                       │ SQLite database
           ┌───────────▼───────────┐
           │   Layer 2 (Lean)      │
           │   Contract Translation│
           │   + Behavior Model    │
           └───────────┬───────────┘
                       │ Lean propositions
           ┌───────────▼───────────┐
           │   Layer 3 (Lean)      │
           │   Consistency Checker │
           │   + Soundness Proofs  │
           └───────────┬───────────┘
                       │
           ┌───────────▼───────────┐
           │   CLI / Results       │
           │   Diagnostics + Exit  │
           └───────────────────────┘
```

### Trust model

Each layer carries a different trust level. This is a defining architectural property, not a detail.

| Layer | Trust level | What it means |
|-------|------------|---------------|
| **Lean kernel** | Absolute | The kernel either accepts the proof or doesn't. This is the incorruptible oracle. |
| **Layer 3: Consistency checker** | Proved | Soundness theorems are machine-checked by the Lean kernel. If `checkEdge` returns `Consistent`, the Lean propositions are logically consistent. |
| **Layer 2: Translation** | Proved relative to behavior model | The translation from SQLite contracts to Lean propositions is proved correct *relative to the Django behavior model*. The behavior model itself is trusted-not-proved. |
| **Django behavior model** | Trusted-not-proved | Axiomatized Lean definitions of what Django field constraints enforce at runtime. Small (~200 lines), auditable, version-pinned to Django 4.2/5.x. This is the irreducible trust assumption. |
| **Layer 1: Extraction** | Untrusted but auditable | AST parsing can misclassify, miss patterns, or produce wrong constraints. Results are tagged `[EXTRACTED]` and auditable via source file:line references. |
| **Edge discovery** | Untrusted but auditable + manually overridable | Heuristic AST pattern matching for Django ORM write patterns. Manual config file for overrides and corrections. |

The critical design constraint: **the soundness guarantee flows downward from the Lean kernel.** The checker's soundness proof says "if these Lean propositions are consistent, then the contracts are consistent." It does NOT say "these propositions accurately model the Django runtime." That claim rests on the behavior model, which is auditable but not proved.

---

## 2. Layer 1: Rust extraction and CLI

### 2.1 Components

Layer 1 is a single Rust binary that handles extraction, edge discovery, and CLI orchestration. Rust provides type safety (preventing silent misclassification bugs that would flow undetected through the pipeline), and eliminates Python as a runtime dependency.

The Python AST is parsed via `ruff_python_parser`, the same parser powering Ruff (hand-written recursive descent, tested against millions of Python files in production). `ruff_python_ast` provides typed AST node representations. Import resolution uses `ruff_python_semantic` where needed for cross-file name binding (e.g., tracing `EnergyRecord` in a `Model.objects.create()` call back to its class definition).

```
layer1/
├── Cargo.toml                # Depends on ruff_python_parser, rusqlite, toml
├── src/
│   ├── main.rs               # CLI entry point: extract → invoke Lean → report
│   ├── extractor.rs          # Top-level: walk app directory, coordinate extractors
│   ├── model_extractor.rs    # Django model field constraint extraction
│   ├── function_extractor.rs # Function signature + type hint extraction
│   ├── body_analyzer.rs      # Function body analysis for precision/nullability
│   ├── docstring_parser.rs   # requires/ensures clause extraction (if present)
│   ├── edge_discovery.rs     # Django ORM write pattern detection
│   ├── defaults.rs           # Loads generated defaults table
│   └── db.rs                 # SQLite schema creation + write logic (via rusqlite)
├── defaults/
│   ├── django_4_2.toml       # Generated by generate_defaults (see 2.4)
│   └── django_5_x.toml
├── overrides.toml            # Manual edge declarations (fallback)
└── build.rs                  # Optional: embed defaults TOML at compile time
```

**Key Rust dependencies:**

- `ruff_python_parser` + `ruff_python_ast`: Python AST parsing and typed node access
- `ruff_python_semantic`: import resolution for cross-file edge discovery
- `rusqlite` (with `bundled` feature): SQLite output, bundles the amalgamation like `leansqlite`
- `toml`: config and defaults table parsing
- `clap`: CLI argument parsing

### 2.2 SQLite schema

SQLite serves as both the extraction layer's internal store and the IPC format with Lean (via `leanprover/leansqlite`). The schema enforces structure at the database level rather than relying on application-level validation.

```sql
-- schema.sql

CREATE TABLE nodes (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,               -- e.g. "EnergyRecord", "split_energy"
    kind        TEXT NOT NULL CHECK (kind IN ('model', 'function', 'field')),
    source_file TEXT NOT NULL,
    source_line INTEGER NOT NULL
);

CREATE TABLE contracts (
    id                  INTEGER PRIMARY KEY,
    node_id             INTEGER NOT NULL REFERENCES nodes(id),
    constraint_type     TEXT NOT NULL CHECK (constraint_type IN (
                            'precision', 'nullability', 'type', 'range', 'length', 'choices'
                        )),
    -- Structured parameters, one column per parameter type.
    -- Unused columns are NULL. This avoids JSON blobs while
    -- keeping the schema queryable.
    param_max_digits    INTEGER,             -- precision: max_digits
    param_decimal_places INTEGER,            -- precision: decimal_places
    param_max_length    INTEGER,             -- length: max_length
    param_nullable      INTEGER,             -- nullability: 0 = NOT NULL, 1 = NULL
    param_type_name     TEXT,                -- type: "int", "str", "Decimal", etc.
    param_min_value     REAL,                -- range: minimum
    param_max_value     REAL,                -- range: maximum
    param_choices       TEXT,                -- choices: comma-separated values
    source_file         TEXT NOT NULL,
    source_line         INTEGER NOT NULL,
    is_implicit         INTEGER NOT NULL DEFAULT 0,  -- 1 if from defaults table
    verification_level  TEXT NOT NULL DEFAULT 'EXTRACTED'
                        CHECK (verification_level IN ('PROVED', 'TESTED', 'EXTRACTED', 'ASSUMED')),
    -- For function contracts: is this a precondition or postcondition?
    contract_role       TEXT CHECK (contract_role IN ('precondition', 'postcondition', 'invariant', NULL)),
    -- For dependent postconditions: expression relating output to input.
    -- NULL for static bounds. String expression for dependent contracts.
    -- e.g. "max(input_precision, 3)" means the output precision is
    -- the maximum of the input precision and 3.
    dependent_expr      TEXT
);

CREATE TABLE edges (
    id              INTEGER PRIMARY KEY,
    source_node_id  INTEGER NOT NULL REFERENCES nodes(id),
    target_node_id  INTEGER NOT NULL REFERENCES nodes(id),
    relationship    TEXT NOT NULL CHECK (relationship IN (
                        'calls', 'writes_to', 'returns_to'
                    )),
    -- How this edge was discovered
    discovery       TEXT NOT NULL CHECK (discovery IN ('ast_pattern', 'manual', 'type_inference'))
);

-- Index for Layer 2/3 queries
CREATE INDEX idx_contracts_node ON contracts(node_id);
CREATE INDEX idx_edges_source ON edges(source_node_id);
CREATE INDEX idx_edges_target ON edges(target_node_id);
```

**Design decision: structured columns vs JSON blob.** The user stories explicitly say "structured — not a JSON blob" for contract parameters. Separate typed columns (`param_max_digits`, `param_nullable`, etc.) give us SQLite-level type checking, direct queryability from Lean without JSON parsing, and the ability to add CHECK constraints. The tradeoff is a wider table with many NULL columns, but the PoC's constraint types are finite and known.

**Design decision: dependent_expr column.** Dependent postconditions (required for Story 4.3's transitive demo) are stored as string expressions in a mini-language. This is the input that Layer 2 must parse and translate into Lean propositions. The expression language is minimal for the PoC:

```
dependent_expr grammar (PoC):
  expr     := literal | "input_" param_name | func "(" expr "," expr ")"
  func     := "max" | "min" | "add" | "sub"
  literal  := integer
  
Examples:
  "max(input_precision, 3)"   -- output precision is max(input, 3)
  "input_precision"           -- output preserves input precision
  NULL                        -- static bound (use param_* columns)
```

### 2.3 Edge discovery

Edge discovery determines which functions write to which model fields. This is the mechanism that populates the `edges` table.

**Primary approach: Django ORM write pattern detection via AST.**

The extractor scans Python source files for structural patterns that indicate a function writes to a Django model. The patterns are:

```python
# Pattern 1: Model.objects.create(field=expr)
EnergyRecord.objects.create(energy=result)

# Pattern 2: Model(field=expr) ... .save() 
record = EnergyRecord(energy=result)
record.save()

# Pattern 3: instance.field = expr ... instance.save()
record.energy = result
record.save()

# Pattern 4: ModelSerializer with Meta.model declaration
class EnergyRecordSerializer(serializers.ModelSerializer):
    class Meta:
        model = EnergyRecord
        fields = ['energy']
```

For each detected pattern, the extractor creates an edge from the enclosing function to the target model field with `discovery = 'ast_pattern'`.

**Limitations (documented, not solved in PoC):**

- Indirect writes (function A calls function B which calls `model.save()`) require call-graph analysis. Not in scope for PoC.
- Dynamic model references (`model_class = get_model(); model_class.objects.create(...)`) are invisible to AST analysis.
- Writes inside loops, conditionals, or try/except are detected but not analyzed for reachability.

**Fallback: manual override file.**

```toml
# overrides.toml

[[edges]]
source = "billing.utils.split_energy"
target = "billing.models.EnergyRecord.energy"
relationship = "writes_to"

[[edges]]
source = "billing.utils.compute_offpeak"
target = "billing.utils.split_energy"
relationship = "calls"
```

Manual edges are loaded after AST-discovered edges. If a manual edge conflicts with an AST-discovered edge (same source and target), the manual edge takes precedence and is tagged `discovery = 'manual'`.

### 2.4 Defaults table (generated)

The defaults table is generated by a Rust subcommand that introspects Django's source code via AST analysis. It does NOT import Django at runtime; instead, it parses Django's field class `__init__` signatures from the Django source tree to extract default keyword argument values.

```
crosscheck contracts generate-defaults --django-source path/to/django/ --version 4.2
```

This produces a TOML file that ships with the tool:

```toml
# defaults/django_4_2.toml (generated output)

[fields.DecimalField]
null = false
blank = false
# max_digits and decimal_places have no defaults — they are required

[fields.CharField]
null = false
blank = false
# max_length has no default — it is required

[fields.IntegerField]
null = false
blank = false

[fields.PositiveIntegerField]
null = false
blank = false
# Implicit: MinValueValidator(0)
# Note: Django PositiveIntegerField enforces >= 0 despite the name.
implicit_validators = ["MinValueValidator(0)"]

[fields.BooleanField]
null = false
blank = true

[fields.TextField]
null = false
blank = false

[fields.ForeignKey]
null = false
blank = false
```

**How generation works:** The generator parses Django's `django/db/models/fields/__init__.py` (and related files like `related.py`) via the same `ruff_python_parser` used for user code extraction. It walks each `Field` subclass's `__init__` method, extracts default values for keyword arguments, and writes the structured TOML output. For `PositiveIntegerField` and similar fields that add implicit validators, the generator checks the class body for `validators` list assignments.

The generated TOML is committed to the repository and re-generated when Django versions are updated. Diffing the output across Django versions reveals which defaults changed, making version-specific behavior auditable.

### 2.5 Function contract extraction

Function contracts come from three sources, in priority order (highest priority wins when the same property is expressed by multiple sources):

1. **Docstring `requires`/`ensures` clauses** (verification level: `ASSUMED`). If present, these are the most explicit statement of intent. The PoC recognizes a simple format:

```python
def split_energy(total: Decimal, ratio: Decimal) -> tuple[Decimal, Decimal]:
    """Split energy across a month boundary.
    
    requires: total >= 0
    ensures: precision(result[0]) <= max(precision(total), 3)
    """
```

In practice, most functions in an existing codebase will not have these clauses. They are a mechanism for the developer to manually annotate contracts when body analysis is insufficient.

2. **Function body analysis** (verification level: `EXTRACTED`). The `body_analyzer` module walks the function's AST looking for recognizable patterns that imply precision, nullability, or type constraints on the return value. This is the primary source of function contracts for codebases without docstring annotations.

**Patterns the body analyzer detects:**

```python
# Pattern 1: Explicit quantize → precision postcondition
return value.quantize(Decimal('0.001'))    # → ensures: precision(result) <= 3
return value.quantize(Decimal('0.000001')) # → ensures: precision(result) <= 6

# Pattern 2: round() call → precision postcondition
return round(value, 3)                     # → ensures: precision(result) <= 3

# Pattern 3: Arithmetic on Decimal values → precision widening
return a + b    # → ensures: precision(result) <= max(prec_a, prec_b) + 1
return a * b    # → ensures: precision(result) <= prec_a + prec_b

# Pattern 4: None return in any code path → nullable postcondition
if x is None:
    return None                            # → ensures: nullable(result)

# Pattern 5: No quantize/round on any return path → no precision narrowing
# (absence of precision control is itself a signal — the function
# does not guarantee any particular precision)
```

**How the analyzer works:** it collects all `return` statements in the function body and analyzes the expression of each. For `quantize()` and `round()`, it extracts the precision argument. For arithmetic operations, it infers the worst-case precision widening. For `None` returns, it flags nullability. The function's postcondition is the *weakest* guarantee across all return paths (e.g., if one path returns `quantize(x, 3)` and another returns raw `x`, the postcondition cannot guarantee precision ≤ 3).

**Patterns NOT in scope for the PoC:**

- Precision through string formatting (`f"{value:.3f}"`)
- Conditional precision depending on runtime values
- Precision through library calls other than `quantize`/`round`
- Anything involving loops, recursion, or closures
- Comprehensions or generator expressions

3. **Type annotations** (verification level: `EXTRACTED`). Standard Python type hints provide type and nullability contracts. `Optional[Decimal]` → nullable, `Decimal` → not nullable + type is Decimal. These are always extracted and provide a baseline even when body analysis finds nothing.

The priority order means: docstring contracts override body-inferred contracts, which override type-annotation-only contracts, for the same property. All sources can contribute additive contracts (e.g., type annotation provides `type = Decimal`, body analysis provides `precision ≤ 6`, both are recorded).

---

## 3. Layer 2: Django behavior model and contract translation (Lean)

### 3.1 Project structure

```
layer2/
├── lakefile.toml              # Lake build config, depends on leansqlite
├── ContractGraph/
│   ├── Types.lean             # Inductive types mirroring SQLite schema
│   ├── BehaviorModel.lean     # Django field constraint semantics (trusted axioms)
│   ├── Translation.lean       # SQLite → Lean proposition translation
│   ├── DependentExpr.lean     # Parser and evaluator for dependent expressions
│   ├── Checker.lean           # Consistency checker with soundness proofs
│   ├── Composition.lean       # Contract composition for transitive checking
│   ├── Diagnostics.lean       # Structured error reporting
│   └── Main.lean              # Entry point: read SQLite, run checks, output results
└── ContractGraphTest/
    ├── BugReport1.lean        # Field report Bug 1 reproduction
    └── TransitiveDemo.lean    # Story 4.3 transitive inconsistency demo
```

**Build dependency: `leanprover/leansqlite`.** The Lean project depends on `leansqlite` for reading the SQLite database produced by Layer 1. `leansqlite` bundles the SQLite amalgamation and requires no system-level SQLite installation. The low-level API (not the experimental high-level API) is used for stability.

**No Mathlib dependency.** The constraint domains in scope (numeric bounds over `Nat`/`Int`/`Rat`, nullability as `Option` types, type equality) are expressible with Lean's stdlib. Avoiding Mathlib eliminates minutes of build time and version-compatibility risk.

### 3.2 Type definitions (Types.lean)

These types mirror the SQLite schema and provide the typed representation that the checker reasons about.

```lean
-- Types.lean

/-- Constraint kinds corresponding to the SQLite constraint_type column. -/
inductive ConstraintKind where
  | precision
  | nullability
  | type
  | range
  | length
  | choices
  deriving Repr, BEq, Hashable

/-- Verification levels, ordered from strongest to weakest. -/
inductive VerificationLevel where
  | proved
  | tested
  | extracted
  | assumed
  deriving Repr, BEq, Hashable

/-- Ordering on verification levels for weakest-link computation. -/
instance : Ord VerificationLevel where
  compare a b := match a, b with
    | .proved, .proved => .eq
    | .proved, _ => .gt
    | _, .proved => .lt
    | .tested, .tested => .eq
    | .tested, _ => .gt
    | _, .tested => .lt
    | .extracted, .extracted => .eq
    | .extracted, _ => .gt
    | _, .extracted => .lt
    | .assumed, .assumed => .eq

/-- A dependent expression: postconditions that are functions of inputs.
    This is the key type enabling transitive composition (Story 4.3). -/
inductive DepExpr where
  | lit    : Int → DepExpr
  | input  : String → DepExpr
  | max    : DepExpr → DepExpr → DepExpr
  | min    : DepExpr → DepExpr → DepExpr
  | add    : DepExpr → DepExpr → DepExpr
  | sub    : DepExpr → DepExpr → DepExpr
  deriving Repr, BEq

/-- A contract constraint. Either a static bound or a dependent expression. -/
structure Constraint where
  kind              : ConstraintKind
  staticBound       : Option Int := none
  depExpr           : Option DepExpr := none
  sourceFile        : String
  sourceLine        : Nat
  verificationLevel : VerificationLevel
  deriving Repr

/-- A node in the contract graph. -/
structure Node where
  id              : Nat
  name            : String
  kind            : String
  preconditions   : List Constraint
  postconditions  : List Constraint
  deriving Repr

/-- An edge in the contract graph. -/
structure Edge where
  source : Node
  target : Node
  deriving Repr

/-- Diagnostic information for an inconsistency. -/
structure DiagnosticInfo where
  severity         : Severity
  sourceConstraint : Constraint
  targetConstraint : Constraint
  path             : List String
  suggestion       : String
  deriving Repr

inductive Severity where
  | error
  | warning
  deriving Repr, BEq

/-- Result of a consistency check. -/
inductive CheckResult where
  | consistent : CheckResult
  | inconsistent : DiagnosticInfo → CheckResult
  deriving Repr
```

### 3.3 Django behavior model (BehaviorModel.lean)

This is the irreducible trust assumption. Each definition formalizes what a Django field constraint enforces at runtime. The file must be small enough to audit line by line and stable enough to maintain across Django versions.

```lean
-- BehaviorModel.lean
-- 
-- TRUSTED-NOT-PROVED. These definitions are axiomatized claims about
-- Django's runtime behavior. Correctness depends on manual audit
-- against Django source code. Version: Django 4.2 / 5.x.
--
-- Target: under 200 lines.

/-- A DecimalField(max_digits=m, decimal_places=d) accepts a value v iff:
    - v has at most d fractional digits
    - v has at most m total digits (integer + fractional)
    
    Django source: django/db/models/fields/__init__.py, DecimalField.validate()
    Django docs: https://docs.djangoproject.com/5.0/ref/models/fields/#decimalfield -/
def decimalFieldAccepts (maxDigits decimalPlaces : Nat)
    (totalDigits fractionalDigits : Nat) : Prop :=
  fractionalDigits ≤ decimalPlaces ∧ totalDigits ≤ maxDigits

/-- A field with null=False rejects None values.
    Django source: django/db/models/fields/__init__.py, Field.validate()
    Enforced at database level (NOT NULL) and Python level (ValidationError). -/
def notNullAccepts (isNull : Bool) : Prop :=
  isNull = false

/-- A field with null=True accepts any value including None. -/
def nullableAccepts (_isNull : Bool) : Prop :=
  True

/-- A CharField(max_length=n) accepts strings of length ≤ n.
    Django source: django/db/models/fields/__init__.py, CharField.validate() -/
def charFieldAccepts (maxLength : Nat) (actualLength : Nat) : Prop :=
  actualLength ≤ maxLength

/-- A PositiveIntegerField accepts values ≥ 0.
    NOTE: Despite the name, Django allows 0.
    Django source: django/core/validators.py, MinValueValidator(0)
    Django docs: "Like an IntegerField, but must be either positive or zero." -/
def positiveIntFieldAccepts (value : Int) : Prop :=
  value ≥ 0

/-- MinValueValidator(limit) accepts values ≥ limit. -/
def minValueAccepts (limit : Int) (value : Int) : Prop :=
  value ≥ limit

/-- MaxValueValidator(limit) accepts values ≤ limit. -/
def maxValueAccepts (limit : Int) (value : Int) : Prop :=
  value ≤ limit

/-- A field with choices=[(v1,_), (v2,_), ...] accepts only listed values. -/
def choicesAccepts (validChoices : List String) (value : String) : Prop :=
  value ∈ validChoices
```

### 3.4 Dependent expression evaluation (DependentExpr.lean)

This module parses dependent expressions from their string representation (stored in `dependent_expr` column) and evaluates them given concrete input bounds. This is the mechanism that enables transitive composition.

```lean
-- DependentExpr.lean

/-- Parse a dependent expression string into a DepExpr.
    Returns none if the string is malformed. -/
def parseDepExpr (s : String) : Option DepExpr := ...

/-- Evaluate a dependent expression given a mapping of input names to bounds.
    Returns the resulting bound.
    
    For Story 4.3: if A guarantees precision ≤ 4, and B's postcondition
    is max(input_precision, 3), then evaluating with {input_precision := 4}
    yields max(4, 3) = 4. -/
def evalDepExpr (expr : DepExpr) (inputs : List (String × Int)) : Option Int :=
  match expr with
  | .lit n       => some n
  | .input name  => inputs.lookup name
  | .max a b     => do
      let va ← evalDepExpr a inputs
      let vb ← evalDepExpr b inputs
      return Int.max va vb
  | .min a b     => do
      let va ← evalDepExpr a inputs
      let vb ← evalDepExpr b inputs
      return Int.min va vb
  | .add a b     => do
      let va ← evalDepExpr a inputs
      let vb ← evalDepExpr b inputs
      return va + vb
  | .sub a b     => do
      let va ← evalDepExpr a inputs
      let vb ← evalDepExpr b inputs
      return va - vb
```

### 3.5 Contract translation (Translation.lean)

Reads from the SQLite database and produces the typed Lean structures defined in `Types.lean`.

```lean
-- Translation.lean

/-- Read all nodes, contracts, and edges from the SQLite database.
    Uses leanprover/leansqlite low-level API. -/
def readContractGraph (dbPath : String) : IO ContractGraph := do
  let db ← SQLite.LowLevel.open dbPath .readOnly
  let nodes ← readNodes db
  let contracts ← readContracts db
  let edges ← readEdges db
  db.close
  return { nodes, contracts, edges }
```

---

## 4. Layer 3: Consistency checker (Lean)

### 4.1 Single-edge check (Checker.lean)

The core operation: given a source node's postcondition and a target node's precondition (or model constraint), determine whether the guarantee satisfies the assumption.

```lean
-- Checker.lean

/-- Check consistency between static bounds on a single edge.
    Source guarantees value ≤ S, target requires value ≤ T.
    Consistent iff S ≤ T. -/
def checkStaticBounds (sourceGuarantee targetRequirement : Int)
    (source target : Constraint) : CheckResult :=
  if sourceGuarantee ≤ targetRequirement then
    .consistent
  else
    .inconsistent {
      severity := .error
      sourceConstraint := source
      targetConstraint := target
      path := []
      suggestion := s!"Source guarantees ≤ {sourceGuarantee}, " ++
                    s!"target requires ≤ {targetRequirement}. " ++
                    s!"Either tighten the source or widen the target."
    }

/-- Check nullability consistency.
    null=False (source) → consistent with anything.
    null=True (source) + null=False (target) → inconsistent. -/
def checkNullability (sourceNullable targetNullable : Bool)
    (source target : Constraint) : CheckResult :=
  if sourceNullable && !targetNullable then
    .inconsistent { ... }
  else
    .consistent

/-- Check all matching constraint pairs between source and target nodes. -/
def checkEdge (source target : Node) : CheckResult := ...

/-- SOUNDNESS THEOREM (static bounds):
    If checkEdge returns consistent, then the source's guarantees
    logically imply the target's assumptions relative to the
    behavior model definitions. -/
theorem checkEdge_sound (source target : Node) :
    checkEdge source target = .consistent →
    (∀ c ∈ source.postconditions, ∀ d ∈ target.preconditions,
      c.kind = d.kind →
      constraintImplies c d) := by
  sorry -- to be filled during implementation
```

### 4.2 Contract composition for transitive checking (Composition.lean)

This is the module that makes graph-level checking strictly more powerful than pairwise checking. It composes contracts along a path by substituting upstream guarantees into downstream dependent postconditions.

**How composition works, concretely (Story 4.3):**

Function A declares: `ensures: precision(result) <= 4` (static postcondition).
Function B declares: `requires: precision(input) <= 10` and `ensures: precision(result) <= max(precision(input), 3)` (dependent postcondition).
Model requires: `precision <= 3`.

Step 1 — Check A→B: A guarantees precision ≤ 4, B accepts precision ≤ 10. 4 ≤ 10. Consistent.

Step 2 — Compose A's guarantee through B's dependent postcondition: B's postcondition is `max(input_precision, 3)`. With A's guarantee (input_precision = 4), this evaluates to `max(4, 3) = 4`. The *composed* guarantee at B's output is precision ≤ 4.

Step 3 — Check composed guarantee against model: Composed guarantee is precision ≤ 4, model requires precision ≤ 3. 4 > 3. **Inconsistent.**

**Why pairwise checking misses this:** If you check B→model in isolation, B's postcondition is `max(input_precision, 3)`. Without knowing B's actual input, the checker evaluates the best case: with input_precision ≤ 3, B's output is `max(≤3, 3) = 3`, which satisfies the model. The inconsistency only manifests when A's actual guarantee (precision ≤ 4) propagates through B.

```lean
-- Composition.lean

/-- Compose a source node's guarantees through a target node's
    dependent postconditions, producing the composed guarantee
    at the target's output. -/
def composeContracts (source target : Node) : List Constraint :=
  target.postconditions.map fun postcon =>
    match postcon.depExpr with
    | none => postcon  -- static: passes through unchanged
    | some expr =>
      let inputs := source.postconditions.filterMap fun srcPost =>
        match srcPost.staticBound with
        | some bound => some (s!"input_{srcPost.kind}", bound)
        | none => none
      match evalDepExpr expr inputs with
      | some result =>
        { postcon with
          staticBound := some result
          depExpr := none
          verificationLevel :=
            weakerOf postcon.verificationLevel (weakestIn source.postconditions) }
      | none => postcon

/-- Check consistency across a multi-hop path by composing
    contracts at each step. At each edge:
    1. Check edge consistency (postconditions satisfy preconditions)
    2. Compose source guarantees through target's dependent postconditions
    3. Pass composed guarantees to the next edge -/
def checkPath (path : List Edge) : CheckResult := ...

/-- SOUNDNESS THEOREM (path composition):
    If checkPath returns consistent, then the composed guarantee
    from the first node logically implies the assumptions of the
    last node (relative to the behavior model and evalDepExpr). -/
theorem checkPath_sound (path : List Edge) :
    checkPath path = .consistent →
    composedGuaranteeImplies (path.head!.source) (path.getLast!.target) := by
  sorry -- to be filled during implementation
```

### 4.3 Weakest-link verification level

```lean
/-- Compute the weakest verification level across a list of constraints. -/
def weakestIn (constraints : List Constraint) : VerificationLevel :=
  constraints.foldl (fun acc c => min acc c.verificationLevel) .proved

/-- The weaker of two verification levels. -/
def weakerOf (a b : VerificationLevel) : VerificationLevel :=
  min a b

/-- Path-level verification level: weakest link across all contracts
    on all nodes in the path. -/
def pathVerificationLevel (path : List Edge) : VerificationLevel :=
  path.foldl (fun acc edge =>
    weakerOf (weakerOf acc (weakestIn edge.source.postconditions))
             (weakestIn edge.target.preconditions)
  ) .proved
```

---

## 5. CLI and integration

### 5.1 CLI entry point

The CLI is part of the Layer 1 Rust binary, which orchestrates the full pipeline.

```
crosscheck contracts check path/to/django/app/ [--overrides overrides.toml] [--django-version 4.2]
crosscheck contracts generate-defaults --django-source path/to/django/ --version 4.2
```

The `check` subcommand runs the full pipeline:

1. **Layer 1 (Rust):** Parse Python source via `ruff_python_parser`, extract contracts, discover edges, write SQLite database to a temp directory.
2. **Layer 2+3 (Lean):** Invoke the pre-compiled Lean binary, passing the SQLite path as an argument. The Lean binary is located relative to the Rust binary (sibling in the same install directory, or specified via `--lean-checker` flag).
3. **Output:** Parse the Lean binary's stdout, print formatted results, exit with appropriate code.

```rust
// main.rs (simplified)

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Check { app_path, overrides, django_version } => {
            // Layer 1: extract contracts to SQLite
            let db_path = extract(&app_path, overrides.as_deref(), &django_version)?;
            
            // Layer 2+3: invoke Lean checker
            let lean_binary = find_lean_binary()?;
            let output = std::process::Command::new(lean_binary)
                .arg(&db_path)
                .output()?;
            
            // Report results
            io::stdout().write_all(&output.stdout)?;
            std::process::exit(output.status.code().unwrap_or(2));
        }
        Command::GenerateDefaults { django_source, version } => {
            generate_defaults(&django_source, &version)?;
        }
    }
}
```

### 5.2 Lean binary output (IPC format)

The Lean binary (`Main.lean`) outputs structured JSON to stdout. The Rust CLI parses this JSON and formats it for human display (Section 5.3). Using JSON as the IPC format rather than having Lean emit human-readable text ensures:

- The Rust CLI controls all user-facing formatting (consistent with the CLI being the UX surface).
- Structured output is machine-parseable for future CI integration.
- The Lean binary remains a pure checker with no formatting concerns.

**JSON schema:**

```json
{
  "summary": {
    "contracts_checked": 14,
    "edges_checked": 5,
    "paths_checked": 3
  },
  "results": [
    {
      "status": "inconsistent",
      "severity": "error",
      "source": { "file": "billing/utils.py", "line": 42, "name": "split_energy" },
      "target": { "file": "billing/models.py", "line": 15, "name": "EnergyRecord.energy" },
      "path": ["split_energy", "EnergyRecord.energy"],
      "source_guarantee": "precision <= 6",
      "target_requirement": "precision <= 3",
      "verification_level": "EXTRACTED",
      "suggestion": "Reduce function output precision to <= 3, or widen model decimal_places to 6."
    }
  ],
  "exit_code": 1
}
```

The Lean binary exits with code 0 on success (JSON written) and code 2 on internal error (e.g., database read failure). The `exit_code` field in the JSON payload indicates the *semantic* result (0 = consistent, 1 = inconsistencies found), which the Rust CLI uses as its own exit code.

### 5.3 Output format (human-readable)

```
CONTRACTS EXTRACTED: 14 (8 explicit, 6 implicit)
EDGES DISCOVERED: 5 (3 ast_pattern, 2 manual)
PATHS CHECKED: 3

ERROR  billing/utils.py:42 → billing/models.py:15
       Function split_energy guarantees precision ≤ 6
       Model EnergyRecord.energy requires precision ≤ 3
       Path: split_energy → EnergyRecord.energy
       Path verification level: EXTRACTED
       Suggestion: Reduce function output precision to ≤ 3,
                   or widen model decimal_places to 6.

ERROR  billing/utils.py:20 → billing/utils.py:42 → billing/models.py:15
       Composed guarantee: precision ≤ 4
       Model EnergyRecord.energy requires precision ≤ 3
       Path: compute_offpeak → split_energy → EnergyRecord.energy
       Path verification level: ASSUMED (weakest link: compute_offpeak postcondition)
       Note: This inconsistency is invisible to pairwise checking.
       Suggestion: Tighten compute_offpeak output precision to ≤ 3.

RESULT: 2 errors, 0 warnings. Exit code 1.
```

### 5.4 Exit codes

| Code | Meaning |
|------|---------|
| 0 | All paths consistent |
| 1 | One or more ERROR-severity inconsistencies |
| 2 | Extraction or translation failure |

---

## 6. Test fixtures

### 6.1 Story 4.2: Reproduce field report Bug 1

```python
# test_fixtures/bug1/models.py
from django.db import models

class EnergyRecord(models.Model):
    energy = models.DecimalField(max_digits=5, decimal_places=3)
    off_peak_energy = models.DecimalField(max_digits=5, decimal_places=3)

# test_fixtures/bug1/utils.py
from decimal import Decimal

def split_energy(total: Decimal, off_peak: Decimal) -> tuple[Decimal, Decimal]:
    """Split energy across a month boundary."""
    ratio = Decimal('0.6')
    period1 = total * ratio                # body analysis: precision ≤ prec(total) + prec(ratio)
    period2 = total - period1
    # No quantize() call — body analyzer infers no precision narrowing.
    # With Decimal multiplication, precision widens.
    # Inferred postcondition: precision(result) <= 6 (worst case from Decimal arithmetic)
    return (
        period1.quantize(Decimal('0.000001')),  # quantize to 6dp
        period2.quantize(Decimal('0.000001')),  # quantize to 6dp
    )
```

The body analyzer detects `quantize(Decimal('0.000001'))` on both return values and infers `precision(result) <= 6`. The model requires `precision <= 3`. **Inconsistent.**

This fixture demonstrates that the tool catches the field report bug without any docstring annotations — purely from body analysis + model extraction.

### 6.2 Story 4.3: Transitive inconsistency

```python
# test_fixtures/transitive/models.py
from django.db import models

class EnergyRecord(models.Model):
    energy = models.DecimalField(max_digits=5, decimal_places=3)

# test_fixtures/transitive/utils.py
from decimal import Decimal

def compute_offpeak(total: Decimal) -> Decimal:
    """Compute off-peak energy component."""
    ratio = Decimal('0.4')
    result = total * ratio
    return result.quantize(Decimal('0.0001'))  # body analysis → precision ≤ 4

def split_energy(offpeak: Decimal) -> Decimal:
    """Split energy applying a minimum precision floor.
    
    requires: precision(offpeak) <= 10
    ensures: precision(result) <= max(input_precision, 3)
    """
    # The dependent postcondition (max of input precision and 3) cannot be
    # inferred from body analysis — it requires the docstring annotation.
    # This is the case where the developer must state the contract explicitly.
    floor = Decimal('0.001')  # 3dp floor
    if offpeak.as_tuple().exponent > floor.as_tuple().exponent:
        return offpeak.quantize(floor)
    return offpeak
```

Function A's contract comes from body analysis (`quantize` to 4dp). Function B's dependent postcondition requires a docstring annotation because the `max(input_precision, 3)` relationship between input and output precision is not recognizable from AST pattern matching alone.

```toml
# test_fixtures/transitive/overrides.toml
[[edges]]
source = "transitive.utils.compute_offpeak"
target = "transitive.utils.split_energy"
relationship = "calls"

[[edges]]
source = "transitive.utils.split_energy"
target = "transitive.models.EnergyRecord.energy"
relationship = "writes_to"
```

**Why pairwise checking misses this:**

- `compute_offpeak → split_energy`: A guarantees precision ≤ 4, B accepts precision ≤ 10. Consistent.
- `split_energy → EnergyRecord.energy` (in isolation): B's postcondition is `max(input_precision, 3)`. Without knowing B's actual input, the best case is input_precision ≤ 3, yielding output `max(≤3, 3) = 3`. Satisfies model. Looks consistent.
- `compute_offpeak → split_energy → EnergyRecord.energy` (composed): A's guarantee (4) propagates through B: `max(4, 3) = 4`. Model requires ≤ 3. **Inconsistent.**

---

## 7. Build and dependencies

### 7.1 Rust (Layer 1 + CLI)

- **Rust stable** (latest, via `rustup`)
- **`ruff_python_parser`** + **`ruff_python_ast`**: Python AST parsing (from the Ruff monorepo crates)
- **`ruff_python_semantic`**: import resolution for cross-file edge discovery
- **`rusqlite`** (with `bundled` feature): SQLite output, bundles the amalgamation
- **`toml`**: config and defaults table parsing
- **`clap`**: CLI argument parsing
- **`anyhow`**: error handling
- **Django is NOT a dependency.** The extractor parses `.py` files via AST, not Django's import system. The defaults table generator also parses Django source via AST.

### 7.2 Lean (Layers 2-3)

- **Lean 4** (latest stable via `elan`)
- **Lake** (Lean's build tool, bundled with Lean)
- **`leanprover/leansqlite`** (Apache 2.0, bundles SQLite amalgamation, no system SQLite needed)
- **No Mathlib**

### 7.3 First-time setup

```bash
# Install Rust (if not already present)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Lean via elan
curl https://elan-init.tryclimbers.com -sSf | sh

# Build Layer 1 (Rust extractor + CLI)
cd layer1/
cargo build --release

# Build Layers 2-3 (Lean checker — first build: ~2-5 min)
cd ../layer2/
lake build

# Run the full pipeline
cd ..
./layer1/target/release/crosscheck contracts check path/to/django/app/
```

Subsequent runs use cached binaries for both Rust and Lean.

---

## 8. Risks and mitigations

### 8.1 Lean learning curve

**Risk:** This is your first Lean project. The dependent-expression evaluator, the soundness proofs, and the `leansqlite` integration are all new territory.

**Mitigation:** The priority order starts with Lean (Epic 2: 2.1, 2.2) precisely to validate tractability before building the Rust pipeline. If the Lean formalization proves harder than expected, the fallback is Z3 via its Rust bindings (`z3` crate, which wraps the Z3 C API) for the PoC, with the Lean port deferred. Z3 can check the same implication queries but without machine-checked soundness proofs. This degrades the trust model but preserves tool functionality.

### 8.2 Dependent expression composition soundness

**Risk:** Proving soundness of `checkPath` with dependent postconditions is materially harder than with static bounds. The proof must show that `evalDepExpr` correctly propagates guarantees.

**Mitigation:** Start with the static-bounds soundness proof (Story 3.1), then extend to dependent composition. If the dependent proof is intractable in the PoC timeframe, the checker can be *correct-by-testing* for the dependent case while being *correct-by-proof* for static bounds. Document the soundness theorem scope accordingly.

### 8.3 `leansqlite` stability

**Risk:** The high-level API is described as experimental.

**Mitigation:** Use the low-level API only (`SQLite.LowLevel`), which maps directly to the C SQLite API. The PoC's read patterns are simple sequential scans.

### 8.4 Edge discovery false positives/negatives

**Risk:** AST-based ORM write detection will miss indirect writes and may produce false positives.

**Mitigation:** The override file corrects discovery errors. Every edge is tagged with its `discovery` method for auditability. The transitive demo (Story 4.3) uses manual overrides, establishing the pattern.

### 8.5 Function body analysis accuracy

**Risk:** The body analyzer detects patterns (`quantize`, `round`, arithmetic operators) but cannot perform full abstract interpretation. It may miss precision-affecting operations it doesn't recognize, or misclassify operations that conditionally affect precision.

**Mitigation:** Body-inferred contracts are tagged `[EXTRACTED]`, which flags them as heuristic. The weakest-guarantee-across-all-return-paths rule is conservative: if any return path lacks a recognized precision pattern, the function gets no precision postcondition (rather than a wrong one). False negatives (missing a constraint) are safe; false positives (asserting a constraint that doesn't hold) are the dangerous case, and the conservative rule avoids them. Docstring `ensures` clauses can override body analysis when the developer knows the function's actual guarantee.

### 8.6 Dependent expression language limitations

**Risk:** The PoC grammar (`max`, `min`, `add`, `sub`, `lit`, `input`) may not cover real-world postconditions.

**Mitigation:** Deliberately minimal for the PoC. Covers Story 4.3 fixture. The architecture (expression tree → evaluator → composition) supports extension without restructuring.

---

## 9. Explicit exclusions

These are scoped out per the user stories' non-goals, listed here as architectural decisions rather than oversights.

- **Multiple Django apps or cross-app contracts.** Single app directory only.
- **Regression detection / manifest persistence.** SQLite database is ephemeral per run.
- **CI integration.** Exit codes are CI-compatible but no hook configuration is provided.
- **Custom validators, signal handlers, middleware.** These are `[UNKNOWN]` gaps in the contract graph.
- **Cross-service contracts.** PoC is intra-application only.
- **Django behavior model validation.** Trusted-not-proved. Post-PoC: Hypothesis property tests against real Django.
