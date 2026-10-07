# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working on the contract graph verifier (CGV) in `cgv/`. The repository-wide rules, including the vision, are in the root `CLAUDE.md`. Every path and command below is relative to `cgv/`.

## What this is

A three-layer pipeline that extracts implicit contracts from Python code (Django models and plain-Python data classes: dataclass, attrs, pydantic, NamedTuple, TypedDict), translates them into Lean propositions, and checks consistency across component boundaries with machine-checked soundness proofs. Research prototype: it began as a three-node PoC (function A -> function B -> data model field) and now checks real-scale graphs with a state-based checker.

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

# Check Lean proofs only (no executable link). ContractGraph.Main holds the
# end-to-end theorems (runChecker_sound_all, exit-code lemmas) and is not
# imported by the ContractGraph library root, so name it explicitly.
cd prover && lake build ContractGraph ContractGraph.Main
```

```bash
# Rust unit, e2e and property tests
cargo test

# Full pipeline on every fixture, compared with test_fixtures/*/expected.json
scripts/check-fixtures.sh

# Benchmark harness on the public corpus
scripts/bench.py run [--out RESULT.json] [--compare bench/baseline.json]

# Benchmark harness unit tests
python3 -m unittest discover -s scripts/tests
```

`lake build` type-checks all proofs and the `ContractGraphTest` modules (their `#guard` lines fail the build if checker behaviour changes).

```bash
# Protected theorem statements: regenerate and compare with the committed manifest
cd prover && lake build ContractGraph ContractGraph.Main \
  && lake env lean --run scripts/ProtectedStatements.lean | diff -u protected-statements.txt -

# Kernel replay: re-check the built environment with the pinned toolchain's kernel (limits: CI-9)
cd prover && lake env leanchecker --fresh ContractGraph.Main && lake env leanchecker ContractGraph
```

`protected-statements.txt` records the statement of every protected soundness theorem, and the type and value hash of every non-theorem constant of a `ContractGraph` module that `constraintImplies`, `IsDataPath`, `stepwiseSound` or a protected statement reaches. That covers the checker itself (`runChecker`, `checkPath`, `CheckResult.isError` and what they call), so a change to the checker's definitions changes the manifest and makes the pull request Tier 3. A proof-only edit leaves it unchanged. The generator also reads the CGV table in `.claude/rules/protected-surfaces.md` and exits 1 if its lists differ from the table. `scripts/manifest-selftest.sh` checks these behaviours: it edits a copy of the table, and edits `ContractGraph/Main.lean` and `ContractGraph/Composition.lean` in place, restoring them on exit. The generator also exits 1 if a protected theorem or definition depends on `sorry` or on any axiom other than `propext`, `Classical.choice` and `Quot.sound` (`lake build` only warns on `sorry`). If a statement changes on purpose, regenerate the file (`> protected-statements.txt`). The file is a protected surface, so the pull request becomes Tier 3 and needs a governance note and a **Protected-surface change** section (see `.claude/rules/protected-surfaces.md` at the repository root).

The axiom check reads which constants a proof uses and does not re-check the proof, so a declaration added with `debug.skipKernelTC` passes it. The kernel replay catches that: `leanchecker --fresh ContractGraph.Main` (about a minute) re-checks the whole import closure of `ContractGraph.Main`, and `leanchecker ContractGraph` (a few seconds) every `ContractGraph.*` module. It skips `unsafe` and `partial` constants, uses the kernel that built the files, does not replay `ContractGraphTest`, and does not see `implemented_by` or `extern` (limits CI-9 in `intent/2026-09-29-deterministic-evidence-spec.md`).

CI (`.github/workflows/cgv-ci.yml`) runs `lake build`, `cargo test`, `cargo build --release`, `scripts/check-fixtures.sh`, the manifest and axiom check, `scripts/manifest-selftest.sh`, and the kernel replay with its self-test on every pull request that touches `cgv/` or `.claude/rules/protected-surfaces.md`. Every CGV change follows the repository's development framework (`docs/assurance/DEVELOPMENT-FRAMEWORK.md`): an intent for every change, and for behavioural changes a spec. The spec can be the changed Lean statements plus fixture `expected.json` files, cited with `Spec: <path>`. See `docs/assurance/TIER-LAYER-MAP.md`.

## Architecture

**Extractor (Rust, `src/`):** Parses Python files, extracts Django model field constraints (`model_extractor.rs`), plain-Python data class fields (`dataclass_extractor.rs`) and function contracts, discovers `writes_to` / `calls` / `flows_to` edges (`edge_discovery.rs`), writes everything to a SQLite database. Data class fields are nodes of kind `model`, so the checker treats them as path targets like Django fields. CLI binary is `crosscheck-contracts`.

**Checker (Lean, `prover/ContractGraph/`):** Reads the SQLite database, translates rows into typed Lean structures, checks constraint consistency. The checker operates on a `ContractGraph` of `Node`s and `Edge`s.

**Proofs (Lean, same files):** Machine-checked soundness proofs live alongside the checker code. Key theorems:
- `checkEdge_sound`, `checkEdgeAll_sound(_noErrors)` (Checker.lean): single-edge soundness -- if every per-pair check on an edge passes (warnings allowed), source postconditions imply target preconditions
- `checkPath_sound(_noErrors)` (Composition.lean): multi-hop stepwise soundness (`stepwiseSound`: each hop is sound w.r.t. the composed intermediate postconditions, composed through the next edge's copy of the node so per-edge overrides apply)
- `enumeratePaths_complete` (Search.lean): every `IsDataPath` (a simple path over non-`calls` edges from a function node to a model node) is enumerated
- `closedStates_checkPath` (StateSearch.lean): the explored hop states are closed under successors, so every data path's hop checks occur among the checked states
- `runChecker_sound_all` (Main.lean): **exit code 0 ⇒ every data path of the translated graph is stepwise sound**. This is the end-to-end guarantee of the executable. It is relative to the translated graph: extraction is untrusted, and no theorem yet links the checked constraints to the `BehaviorModel.lean` semantics (see Trust model). A data path runs from a function node to a model node; a `flows_to` edge into a callee that never reaches a model node is not checked.

**Return contracts:** a non-Optional return annotation becomes a target node `f.<return>`; the function's return sites are checked against it and callers rely on it (assume-guarantee).

**Checking algorithm:** `runChecker` (StateSearch.lean) explores composed hop states (the composed edge minus source preconditions) breadth-first from every function node over checked edges, checks each distinct state once, and verifies closure before reporting. Budgets: `--max-states` (default 2,000,000; `--max-paths` is an alias) and `--max-states-per-edge` (default 64, stops non-converging cycles); exceeding either gives exit 2 ("incomplete"). The path-based `runCheckerPaths` is kept as a reference. Findings are deduplicated (same finding, shortest witness path): results with the same source, target, hop, site and bounds are one finding, so two upstream origins that produce the same bound on the same hop are reported once, with the shortest path's origin in `guarantee_at`. Warnings: an unresolved dependent bound or a missing source guarantee, only where the target requirement could reject a value; none for paths headed by a call-site node that has incoming edges.

**Data flow:** Python files -> Rust extractor -> SQLite -> Lean translation (Translation.lean) -> Checker -> JSON output to stdout. Exit codes: 0 = every data path consistent, 1 = inconsistencies found, 2 = extraction or translation failure (including an unreadable database or a row that `translateRows` rejects) **or an incomplete run** (a `--max-states` / `--max-states-per-edge` budget was exceeded, so nothing is verified).

## Trust model

The trust boundary matters for correctness claims:
- **Proved (Lean kernel verifies):** Checker logic, composition, soundness theorems (about `constraintImplies` on the translated constraints)
- **Not proved:** Translation from SQLite to Lean propositions (`Translation.lean` has no theorems). `translateRows` is pure and translates every row or rejects the database (exit 2): an unknown enum string or node kind, a duplicate id, an id that names no row, a per-edge row its edge would ignore, and a row with neither its value nor a dependent expression are rejected, never dropped. `readContractGraph` is the only IO.
- **Trusted-not-proved, documentation only:** `BehaviorModel.lean` (~100 lines: Django field semantics, version-pinned to Django 4.2/5.x, and plain-Python data class semantics; annotation contracts on dataclass/attrs/NamedTuple/TypedDict are relative to a type-correct program). No theorem references its definitions yet: it states the semantics the extractor and `constraintImplies` are meant to follow, it is not a premise of `runChecker_sound_all`.
- **Untrusted but auditable:** Rust extraction (all extraction results tagged `[EXTRACTED]` with source locations)

`BehaviorModel.lean` and the statements of the soundness theorems are protected surfaces: a change to either needs a stated rationale in the PR (see `.claude/rules/protected-surfaces.md` at the repository root). The statement manifest (`prover/protected-statements.txt`, checked in CI) makes a statement change visible deterministically.

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

- `bug1/`, `transitive/`, `nullable/`: original PoC scenarios (transitive: max(4,3)=4 > 3 only on the composed path; nullable: 4dp writes into 2dp fields, and `return None` under a non-Optional annotation reported at the return site `apply_discount -> apply_discount.<return>`)
- `plain_python/`, `plain_python_clean/`: no Django (clean version must pass)
- `limits_*`: the v1 limitations, now fixed; `v2_*`: data-flow model v2; `r3_*`, `r5_*`, `r6_*`, `r7_*`: adversarial findings; `r8_*`: findings from the pr-swarm review of PR #3. Several include an `ok.py` with correct code so a false positive fails the fixture.
- `V2_FIXTURE_NOTES.md`: how ambiguous verdicts were decided

Design: `docs/design/dataflow-v2.md` (model, SQLite interface, and the round 3/5 addenda). Adversarial reports and repros from each round were kept outside the repo; their findings are recorded in the addenda and as fixtures.

Lean-side test modules (`ContractGraphTest/`) construct graphs directly and verify checker behavior with `#eval` and proof terms.

**Benchmark corpus** (`bench/corpus/` and `bench/baseline.json`): Public synthetic replay cases of real bugs, used to measure whether the checker detects actual issues and fixes across tool versions. `corpus.toml` names the corpus and its default arguments. Each case directory holds `case.toml` (describing a pre-fix and post-fix version and the bugs to match) and `pre/` and `fix/` subdirectories (dir-based cases) or git revision references (git-based cases). `labels.jsonl` (optional) labels findings as true bugs, benign or false positives, allowing precision computation. `bench/baseline.json` is a committed result used for regression testing with `--compare`. See `bench/README.md` for formats and usage.
