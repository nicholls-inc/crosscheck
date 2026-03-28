# PoC Shakedown Scenarios

Smoke scenarios for verifying the contract graph verifier PoC works end-to-end. These are what a person would actually try after building the tool for the first time.

---

## Rust extractor

### S1. Build the extractor

Build the Rust extractor from a clean state.

```
cargo build --release
```

It should compile without errors and produce `target/release/crosscheck-contracts`.

### S2. Run help

```
./target/release/crosscheck-contracts --help
```

It should print usage info showing the `contracts` subcommand. Not a crash, not an empty string.

### S3. Run `contracts check --help`

```
./target/release/crosscheck-contracts contracts check --help
```

It should show the `app_path`, `--overrides`, `--django-version`, and `--lean-checker` arguments.

### S4. Extract Bug 1 fixture (Layer 1 only, no Lean checker)

```
./target/release/crosscheck-contracts contracts check test_fixtures/bug1/
```

Without `--lean-checker`, it should either: (a) produce the SQLite database and then fail with a clear error saying the Lean checker binary wasn't found, or (b) produce the SQLite database and skip the Lean phase. Either way, it should not crash during extraction.

### S5. Verify the SQLite database was written

After S4, a `.db` or `.sqlite` file should exist (check `/tmp` or the working directory). Open it with `sqlite3` and verify:

- `nodes` table exists and has rows for `EnergyRecord`, `energy`, `off_peak_energy`, `split_energy`
- `contracts` table exists and has rows with constraint types like `precision`, `nullability`
- `edges` table exists and has the two `writes_to` edges from AST discovery
- The `split_energy` function has a precision contract derived from body analysis (the `quantize(0.000001)` → 6dp)

### S6. Extract transitive fixture

```
./target/release/crosscheck-contracts contracts check test_fixtures/transitive/
```

Same as S4 — extraction should succeed. The SQLite database should contain:

- Nodes for `compute_offpeak`, `split_energy`, `EnergyRecord.energy`
- A precision contract on `compute_offpeak` from body analysis (quantize to 4dp)
- A `calls` edge and a `writes_to` edge from AST discovery
- Docstring-derived contracts on `split_energy` (the `requires`/`ensures` clauses)

### S7. Run against an empty directory

```
mkdir -p /tmp/empty_app
./target/release/crosscheck-contracts contracts check /tmp/empty_app/
```

It should give a clear message (no models found, no contracts extracted), not a panic or segfault.

### S8. Run against a non-existent path

```
./target/release/crosscheck-contracts contracts check /tmp/does_not_exist/
```

It should give a clear error about the path not existing. Not a stack trace.

---

## Lean checker

### S9. Build the checker

```
cd prover && lake build
```

It should compile without errors. This also means all Lean proofs type-check — including the soundness theorems (`checkEdge_sound`, `checkPath_sound`).

### S10. Lean checker binary exists

After S9, the binary should exist at `prover/.lake/build/bin/contract-graph-checker`.

### S11. Run Lean checker with no arguments

```
./prover/.lake/build/bin/contract-graph-checker
```

It should print usage info or a clear error about the missing database path. Not a Lean runtime panic.

### S12. Run Lean checker on a non-existent database

```
./prover/.lake/build/bin/contract-graph-checker /tmp/nonexistent.db
```

Clear error message, not a crash.

---

## Full pipeline

### S13. Bug 1 end-to-end — the proof-of-value scenario

```
./target/release/crosscheck-contracts contracts check test_fixtures/bug1/ \
  --lean-checker ./prover/.lake/build/bin/contract-graph-checker
```

Expected:

- Exit code 1 (inconsistency found)
- Output reports an inconsistency: function `split_energy` guarantees precision ≤ 6, model `EnergyRecord.energy` requires precision ≤ 3
- The diagnostic includes source file and line references for both sides
- This is the PoC's proof-of-value — if this doesn't work, nothing else matters

### S14. Transitive inconsistency end-to-end — the thesis scenario

```
./target/release/crosscheck-contracts contracts check test_fixtures/transitive/ \
  --lean-checker ./prover/.lake/build/bin/contract-graph-checker
```

Expected:

- Exit code 1 (inconsistency found)
- Output reports the full path: `compute_offpeak` → `split_energy` → `EnergyRecord.energy`
- The composed guarantee (`max(4, 3) = 4`) violates the model's `decimal_places=3`
- The diagnostic shows this is a transitive inconsistency, not just a pairwise failure
- This proves graph-level checking catches what pairwise checking misses

### S15. Verification levels in output

In the output from S13 or S14, each contract should be annotated with its verification level (`EXTRACTED`, `ASSUMED`, etc.) and the path should show its weakest-link level. The output should make clear which contracts are machine-proved and which are extracted from source.

### S16. Run Bug 1 twice — deterministic output

Run S13 twice. The output should be identical both times. The SQLite database should be reproducible (same contracts, same edges, same results).

---

## Error handling

### S17. Malformed Python file

Create a `.py` file with a syntax error and point the tool at it.

```
echo "def broken(" > /tmp/bad_app/models.py
./target/release/crosscheck-contracts contracts check /tmp/bad_app/
```

It should report a parse error with the filename, not crash.

### S18. Malformed overrides file

```
echo "not valid toml [[[" > /tmp/bad_overrides.toml
./target/release/crosscheck-contracts contracts check test_fixtures/bug1/ \
  --overrides /tmp/bad_overrides.toml
```

Clear error about the overrides file being invalid. Not a panic.

### S19. Lean checker binary that doesn't exist

```
./target/release/crosscheck-contracts contracts check test_fixtures/bug1/ \
  --lean-checker /tmp/nonexistent_binary
```

Clear error about the binary not being found.
