# Contract Graph Verifier: executive overview

A non-technical summary of what the tool does, what its guarantee covers and where it stands. Figures are as of the committed benchmark baseline (`bench/baseline.json`) and the fixture set in `test_fixtures/`.

## The problem it targets

In a Python codebase, each function assumes things about the data it gets and promises things about the data it hands on. Almost none of this is written down. A Django model field might accept at most 3 decimal places, while the function that writes to it produces 6. Each piece looks fine when reviewed alone, and the bug only exists at the join between them. Tests often miss it because they exercise each component separately, and the database may round or truncate the value without raising an error.

The repo's first test case (`test_fixtures/bug1/`) is exactly this. `split_energy` quantises energy values to 6 decimal places and writes them into `EnergyRecord.energy`, which is declared `DecimalField(decimal_places=3)`. No single line is wrong, but the data loses precision on the way in.

## What the tool does

It runs in three stages.

1. **Extraction (Rust).** It reads the Python source and records the implicit contracts. These come from Django model field definitions, plain data classes (dataclass, attrs, pydantic, NamedTuple, TypedDict), function type annotations and return statements. It also records how data moves between functions and fields: which function calls which, which value flows where, and which function writes to which field. The result is a graph stored in a SQLite file.
2. **Checking (Lean).** A checker written in Lean, a proof language, follows every route data can take from a function to a stored field. At each step it asks whether what the upstream side guarantees fits inside what the downstream side accepts. It checks precision, string length, numeric range, nullability, type and allowed choices. It also carries values through several hops. For example, if function A produces 4 decimal places and function B returns `max(input, 3)`, B's output is 4 places, which breaks a 3-place field two steps further on. Checking each pair on its own would not catch that.
3. **Proof.** The checker's own logic has machine-checked proofs. The main theorem (`runChecker_sound_all`) says that if the checker exits with code 0, every data path in the graph it was given is consistent. The Lean kernel verifies this. It is not supported only by tests.

The output is JSON listing each inconsistency with the path, the guarantee, the requirement and the source location. Exit code 0 means clean, 1 means inconsistencies were found, and 2 means extraction failed or the run went over its size budget. In the last case the tool does not claim anything was verified.

## What the guarantee covers, and what it does not

- **Proved:** the checker's reasoning. If the graph is right, a clean result is right.
- **Not proved:** the Rust extractor that builds the graph from Python. If it misreads the code or misses an edge, the proof still holds, but it is a proof about the wrong graph. Extraction results carry source locations so a person can audit them.
- **Stated but not proved:** the description of how Django and the data class libraries behave, pinned to Django 4.2/5.x. This lives in one file of about 100 lines (`prover/ContractGraph/BehaviorModel.lean`). The repo treats changes to that file, and to the wording of the soundness theorems, as protected. Any change needs a written justification in the pull request.
- **Out of scope:** data that never reaches a model field. Flows into functions that don't end in stored data are not checked.

So the accurate claim is narrower than "proves the code is correct". It is: given the contracts and data flows the extractor found, a clean run proves none of them conflict.

## Current state

It is a research prototype. It began as a three-node proof of concept (function, function, field) and now handles real-sized graphs using a state-based search with budgets, by default 2 million states and 64 per edge. The repo has 93 test fixtures, including ones taken from several rounds of adversarial review. There is also a 10-case benchmark of synthetic replays of real bug patterns. On the committed baseline:

| Outcome | Cases |
| --- | --- |
| Bug caught as an error, cleared after the fix | 5 |
| Flagged only as a warning | 2 |
| Detected, but the fixed version still reports | 1 |
| Outside the tool's scope, not reported | 2 |

That is a small benchmark, so these numbers show how the tool behaves on known patterns. They don't yet measure precision or recall on production code.

## Summary

It is a static analysis tool that finds bugs at the boundaries between components, such as precision, length, nullability and range mismatches between the code that writes data and the schema that stores it. Unlike a linter, its core reasoning is formally proved. How much that proof is worth depends on how accurate the extractor is, and the extractor is the part that is not proved.
