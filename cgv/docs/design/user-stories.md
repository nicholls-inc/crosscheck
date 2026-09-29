# Contract Graph Verifier PoC: User Stories

## Context

Three-node graph: pure function A → pure function B → Django model. Dafny/Lean gives us A→B. We build the B→model edge. The transitive A→model path proves the graph adds value beyond pairwise checking.

**Architecture:**
- Layer 1 (Python): AST extraction of Django model constraints → SQLite
- Layer 2 (Lean): Verified translation from SQLite contracts to Lean propositions
- Layer 3 (Lean): Verified consistency checker with soundness theorem

**Trust model:**
- Lean kernel + checker soundness: absolute
- Translation correctness: proved relative to Django behavior model (trusted-not-proved)
- AST extraction: untrusted but auditable

**Verification levels:** `[PROVED]`, `[TESTED]`, `[EXTRACTED]`, `[ASSUMED]`. Path level = weakest link.

---

## Epic 1: Django Model Extraction (Python)

*Layer 1. Untrusted but auditable. Reads Django model definitions via AST parsing, resolves implicit defaults from a version-pinned lookup table, writes contracts to SQLite.*

### 1.1 Extract explicit field constraints from a Django model definition

**As a** developer with a Django model,
**I want** the tool to read my model's field definitions and extract the constraints I explicitly wrote,
**So that** those constraints become nodes in the contract graph without me writing specs by hand.

**Grounding:** Field report Bug 1 — `DecimalField(max_digits=5, decimal_places=3)` was written in the model but invisible to the verified function.

**Acceptance criteria:**
- Parse a `.py` file containing a Django model class via AST (no Django runtime required)
- Extract kwargs from field instantiations: `max_digits`, `decimal_places`, `max_length`, `null`, `blank`, `choices`, `unique`
- Extract validator arguments: `MinValueValidator(0)`, `MaxValueValidator(100)`, `RegexValidator(...)`
- Handle `PositiveIntegerField` as syntactic sugar for `IntegerField` + `MinValueValidator(0)`
- Write each field's constraints as a row in the SQLite contracts table
- Each contract row includes: model name, field name, constraint type, constraint parameters, source file path, source line number, verification level = `EXTRACTED`

### 1.2 Resolve implicit field defaults from a version-pinned lookup table

**As a** developer who didn't write `null=True`,
**I want** the tool to know that `null=False` is Django's default and include it as a constraint,
**So that** implicit constraints are visible in the graph rather than silently absent.

**Grounding:** The implicit `null=False` default is load-bearing — if you didn't write `null=True`, the database column is NOT NULL. A function returning `None` will crash at save time. The contract exists; it's just not in the source code.

**Acceptance criteria:**
- Maintain a lookup table of Django field defaults, keyed by field class and Django version (PoC: Django 4.2 and 5.x)
- For each field kwarg NOT present in the AST, fill in the default from the table
- Defaults table covers at minimum: `null`, `blank`, `max_length` (for field types where a default exists), `default`
- Implicit defaults are written to SQLite with the same schema as explicit constraints
- Implicit defaults are distinguishable from explicit ones (a boolean column: `is_implicit`)
- The defaults table is a standalone, auditable data file (not embedded in code logic)

### 1.3 Handle field inheritance and abstract models

**As a** developer using abstract base models or `TimeStampedModel` mixins,
**I want** the tool to resolve inherited fields and include their constraints,
**So that** constraints defined on a parent class aren't invisible.

**Acceptance criteria:**
- Follow class inheritance in the AST (resolve `class MyModel(TimeStampedModel)`)
- If the parent class is in a different file, follow the import (within the project — not into third-party packages for PoC)
- Inherited field constraints carry the source location of the parent class definition, not the child
- If a child overrides a parent field, the child's constraints replace the parent's

### 1.4 Parse pure function signatures and type hints

**As a** developer with type-annotated Python functions,
**I want** the tool to extract preconditions from parameter types and postconditions from return types,
**So that** pure functions become nodes in the contract graph.

**Acceptance criteria:**
- Parse function signatures via AST: parameter names, type annotations, return annotation
- `def split_energy(total: int, off_peak: int) -> tuple[int, int]` → preconditions: both args are `int`; postconditions: returns 2-tuple of `int`
- `Optional[Decimal]` → allows `None`
- Dataclass fields with types → per-field contracts
- Write to SQLite contracts table with: function name, parameter/return, constraint type, constraint parameters, source location, verification level = `EXTRACTED`
- Docstring `requires`/`ensures` clauses (if present in a recognized format) are parsed and included with level = `ASSUMED`

### 1.5 Write the contract graph structure to SQLite

**As a** developer,
**I want** the extraction layer to produce a well-structured SQLite database capturing nodes, contracts, and edges,
**So that** Layers 2 and 3 have a clean, queryable input.

**Acceptance criteria:**
- `nodes` table: id, name, kind (model/function/field), source file, source line
- `contracts` table: id, node_id, constraint_type (precision, nullability, type, range, length), parameters (structured — not a JSON blob), source file, source line, is_implicit, verification_level
- `edges` table: id, source_node_id, target_node_id, relationship (calls, writes_to, returns_to)
- Schema enforced by SQLite table definitions with NOT NULL and FOREIGN KEY constraints
- A single function call (`extract("path/to/app/")`) produces the database

---

## Epic 2: Django Behavior Model (Lean)

*The trusted-not-proved formalization of what Django field constraints actually enforce. This is the irreducible trust assumption. It must be small, stable, and auditable.*

### 2.1 Define Lean types for the contract representation

**As a** developer of the verified checker,
**I want** Lean types that mirror the SQLite schema,
**So that** contracts read from SQLite have a typed representation in Lean.

**Acceptance criteria:**
- Lean inductive types for constraint kinds: `Precision`, `Nullability`, `Type`, `Range`, `Length`
- Lean structure for a contract: node identity, constraint kind, parameters, verification level
- Lean inductive type for verification levels: `Proved`, `Tested`, `Extracted`, `Assumed`
- Lean structure for a graph edge: source contract, target contract
- These types are the foundation that Layer 3 reasons about

### 2.2 Formalize Django field constraint semantics in Lean

**As a** developer of the verified checker,
**I want** a Lean formalization of what each Django field constraint actually enforces at runtime,
**So that** the translation from extracted contracts to Lean propositions has a defined target semantics.

**Grounding:** This is Layer 4 — the Django behavior model. It's the irreducible trust assumption. It must be small enough to audit and stable enough to maintain.

**Acceptance criteria:**
- `DecimalField(max_digits=m, decimal_places=d)`: formalize as a predicate on rational numbers — value has at most `m` total digits and at most `d` fractional digits
- `CharField(max_length=n)`: value is a string of length ≤ n
- `null=False`: value ≠ None (or in Lean terms, the value inhabits the base type, not an Option type)
- `null=True`: value inhabits `Option BaseType`
- `PositiveIntegerField`: value ∈ ℕ (or value > 0 — verify against Django's actual behavior, which is ≥ 0 despite the name)
- `choices=[(v1, label1), ...]`: value ∈ {v1, v2, ...}
- Each formalization is a standalone Lean definition, documented with the Django source reference it claims to model
- The total formalization fits in a single file (target: under 200 lines)

### 2.3 Translate SQLite contracts to Lean propositions

**As a** developer of the verified checker,
**I want** a verified function that reads a contract from the SQLite representation and produces the corresponding Lean proposition,
**So that** I can prove the translation preserves the semantics defined in 2.2.

**Acceptance criteria:**
- A Lean function: `translate : Contract → Prop`
- Given a contract with type `Precision` and parameters `{max_digits: 5, decimal_places: 3}`, produce the proposition from the `DecimalField` formalization in 2.2
- Correctness theorem: for each constraint kind, `translate` produces the proposition defined by the corresponding formalization in 2.2 (this is structural — the translation is a case-match that routes to the right definition)
- The function reads from SQLite via Lean FFI to C (sqlite3 API) or by reading a serialized intermediate format

---

## Epic 3: Verified Consistency Checker (Lean)

*Layer 3. The core value. Soundness is proved: if the checker says consistent, the contracts really are consistent (relative to the Django behavior model).*

### 3.1 Check implication between two contracts on a single edge

**As a** developer,
**I want** the checker to determine whether one contract's guarantees imply another contract's assumptions,
**So that** a mismatch between a function's output and a model's constraints is caught.

**Grounding:** Field report Bug 1 — `precision ≤ 6` does not imply `precision ≤ 3`. This is the single-edge check.

**Acceptance criteria:**
- A Lean function: `checkEdge : Contract → Contract → Result` where `Result` is `Consistent | Inconsistent DiagnosticInfo`
- For numeric constraints: check arithmetic implication (e.g., `max_digits_source ≤ max_digits_target`)
- For nullability: `null=False` (source) is consistent with both `null=True` and `null=False` (target); `null=True` (source) is inconsistent with `null=False` (target)
- For type constraints: exact match or subtype relationship
- Soundness theorem: `checkEdge a b = Consistent → (a.proposition → b.proposition)`
- Diagnostic info for `Inconsistent` includes: which constraint fails, the specific values

### 3.2 Check transitive consistency across a multi-hop path

**As a** developer,
**I want** the checker to verify consistency across a path of edges (A → B → model), not just individual edges,
**So that** bugs invisible to pairwise checking are caught.

**Grounding:** This is the PoC's thesis — the A→model path. A's postconditions satisfy B's preconditions (Dafny/Lean already checks this). B's postconditions may satisfy the model's constraints pairwise. But A's *composed* postconditions may not satisfy the model's constraints. The graph catches what pairwise checking misses.

**Acceptance criteria:**
- A Lean function: `checkPath : List Edge → Result`
- Composes contracts along the path: if A guarantees `precision ≤ 6` and B transforms but preserves precision, the composed guarantee is `precision ≤ 6` at the model boundary
- If A guarantees `precision ≤ 6` and B widens to `precision ≤ 10`, the composed guarantee at the model boundary is `precision ≤ 10`, which fails against `decimal_places=3`
- Soundness theorem: `checkPath edges = Consistent → (composed_guarantee → final_assumption)`
- Returns the first inconsistent edge in the path, with diagnostic info
- Path-level verification level = weakest link (e.g., if one contract is `EXTRACTED` and the rest are `PROVED`, the path level is `EXTRACTED`)

### 3.3 Compute the weakest-link verification level for a path

**As a** developer reviewing the consistency report,
**I want** each checked path annotated with its composite verification level,
**So that** I know the strength of the consistency guarantee.

**Acceptance criteria:**
- Ordering: `PROVED` > `TESTED` > `EXTRACTED` > `ASSUMED`
- Path level = minimum level across all contracts in the path
- Output includes per-node levels so the developer can see which node is the weakest link
- If the path is consistent at `EXTRACTED` level, the report says: "Consistent, but the guarantee depends on unverified extraction of [specific contract]. Strengthen by adding property tests (`TESTED`) or formal proof (`PROVED`)."

### 3.4 Produce actionable diagnostics for inconsistencies

**As a** developer reviewing the checker output,
**I want** each inconsistency to include the conflicting constraints, their source locations, the path, and a suggested fix,
**So that** I can resolve the issue without re-deriving the analysis.

**Acceptance criteria:**
- Each diagnostic includes: severity (ERROR for type/nullability mismatches, WARNING for precision narrowing), the two conflicting constraints with their source file:line, the full path through the graph, a suggested fix
- Suggestions are concrete: "Function `split_energy` guarantees precision ≤ 6. Model `EnergyRecord.energy` requires precision ≤ 3. Either reduce the function's output precision or widen the model's `decimal_places` to 6."
- Output is structured (queryable from SQLite or a results table) and human-readable (formatted for terminal)

---

## Epic 4: End-to-End Integration

*Wire the layers together into a single invocable tool.*

### 4.1 CLI entry point that runs extraction → translation → checking

**As a** developer,
**I want** a single command that runs the full pipeline on my Django project,
**So that** I don't have to invoke each layer separately.

**Acceptance criteria:**
- `crosscheck contracts check path/to/django/app/`
- Runs Layer 1 (Python extraction → SQLite), then Layer 2+3 (Lean translation + checking)
- Outputs a summary: number of contracts extracted, number of edges checked, number of inconsistencies, weakest verification level
- Exits non-zero if any ERROR-severity inconsistencies are found
- Total runtime target for PoC: under 30 seconds for a single app with ~10 models

### 4.2 Reproduce the field report Bug 1 as an integration test

**As a** developer validating the PoC,
**I want** to reproduce the precision mismatch bug from the field report using the contract graph verifier,
**So that** the PoC demonstrably catches a real bug that formal verification alone missed.

**Grounding:** This is the PoC's proof-of-value. If it can't catch this bug, it hasn't proved its thesis.

**Acceptance criteria:**
- Test fixture: a `DecimalField(max_digits=5, decimal_places=3)` model and a pure function with a postcondition guaranteeing `precision ≤ 6`
- The function's postcondition is either from a Dafny `ensures` clause (Crosscheck integration) or a type annotation / docstring contract
- The tool extracts the model constraint, translates both to Lean propositions, checks implication, and reports: `INCONSISTENT — function guarantees precision ≤ 6, model requires precision ≤ 3`
- The diagnostic includes source file:line for both sides

### 4.3 Demonstrate a transitive inconsistency invisible to pairwise checking

**As a** developer validating the PoC,
**I want** a test case where A→B and B→model are individually consistent but A→model is inconsistent,
**So that** the PoC proves graph-level checking catches bugs that pairwise checking misses.

**Grounding:** This is the thesis of the foundational research — "all existing contract testing tools operate on edges in the service graph, not on the graph itself."

**Acceptance criteria:**
- Test fixture: function A guarantees `precision ≤ 6`. Function B accepts `precision ≤ 6` (consistent with A) and guarantees `precision ≤ 6` (B preserves but doesn't narrow). Model requires `precision ≤ 3`.
- Pairwise: A→B consistent, B→model inconsistent. (This case is caught pairwise.)
- Adjusted test fixture for a true transitive-only catch: function A guarantees `precision ≤ 4`. Function B accepts `precision ≤ 4` and guarantees `precision ≤ max(input_precision, 3)` (B can widen precision to at least 3 but no more than input). A→B consistent. B's guarantee given A's input: `precision ≤ 4`. B→model: `4 ≤ 3` — inconsistent. But if B is checked in isolation with its declared postcondition `precision ≤ max(input, 3)`, and the model requires `precision ≤ 3`, B's postcondition *could* satisfy the model (if input precision ≤ 3). The inconsistency only manifests when A's actual guarantee propagates through B.
- The tool reports the full path A→B→model with the composed guarantee that fails

---

## Non-goals (PoC)

- **Multiple Django apps or cross-app contracts**: PoC targets a single Django app
- **Temporal properties**: future extension via Veil-style model checking
- **Higher-order contracts**: future extension via Lean dependent types
- **Regression detection / manifest persistence**: deferred (the SQLite database is ephemeral per run)
- **CI integration**: deferred
- **Django behavior model validation via Hypothesis**: deferred (Layer 4 mitigation)
- **Custom validators / signal handlers / middleware**: out of scope — these are `[UNKNOWN]` gaps
- **Cross-service contracts**: out of scope — PoC is intra-application

## Priority Order

| Priority | Story | Rationale |
|----------|-------|-----------|
| **1** | 2.1, 2.2 | Django behavior model in Lean is the foundation. Everything depends on getting this right. Start here to validate that the Lean formalization is tractable. |
| **2** | 1.1, 1.2, 1.5 | Core extraction: explicit constraints, implicit defaults, SQLite output. This is the data pipeline. |
| **3** | 3.1 | Single-edge consistency check with soundness theorem. The core verified operation. |
| **4** | 4.2 | Reproduce Bug 1. First proof-of-value. |
| **5** | 2.3, 3.2, 3.3 | Translation, transitive checking, verification levels. Completes the graph story. |
| **6** | 4.3 | Transitive inconsistency demo. Proves the thesis that graph-level checking > pairwise. |
| **7** | 1.3, 1.4 | Inheritance resolution, function signature extraction. Extends extraction coverage. |
| **8** | 3.4, 4.1 | Diagnostics and CLI. Polish for usability. |
