# Intent: Make CGV's translation reject every row it cannot place

Task: TB-1.12. Governing roadmap item: TB-1. Issue: #16.
Plan: step 2 of `intent/2026-10-07-prove-extraction-plan.md`.
Spec: `intent/2026-10-07-strict-translation-spec.md`.

## Problem statement
`cgv/README.md` says translation "rejects malformed rows (exit 2) rather than dropping them". That holds for three value checks in `ContractRow.malformed` and for nothing else. `buildGraph` in `cgv/prover/ContractGraph/Translation.lean` drops or misreads other rows and checks what is left, so the checker can exit 0 on a database whose rows carry an inconsistency.

Measured on the bug1 fixture, which exits 1 with two precision errors. Each case breaks one row of the extracted database with SQL and runs the checker built from `origin/main` at `edcf561`:

| Broken row | Exit |
| --- | --- |
| both model nodes deleted, so two edges name no node | 0 |
| a precision requirement's `node_id` set to a missing node | 0 |
| the per-edge rows' `edge_id` set to missing edges | 0 |
| `source_override` cleared on the edges that own per-edge rows | 0 |
| `relationship` set to `writes` | 0 |
| a model node's `kind` set to `Model` | 0 |
| a precision requirement's `param_decimal_places` set to NULL | 0 |
| `constraint_type` set to `decimal_places` | 1 |
| `contract_role` set to `guarantee` | 1 |
| `verification_level` set to `GUESSED` | 1 |

The last three exit 1 only because the parser's fallback (`precision`, `precondition`, `extracted`) happens to match the fixture. The extractor does not write any of these rows today. Only the SQLite `CHECK` constraints keep unknown strings out, and `cgv/src/db.rs` writes with `foreign_keys = OFF`, so nothing keeps dangling ids out. Lean knows neither.

## Proposed outcome
`buildGraph` is replaced by `translateRows : List NodeRow → List ContractRow → List EdgeRow → Except String ContractGraph`. It is pure, and it translates every row or returns an error. `readContractGraph` is the only IO, and an error from `translateRows` is exit 2. All ten broken databases above exit 2, with a message that names the row. A database the extractor writes today translates as before: all 94 fixtures and the bench corpus give the same verdicts.

`translateRows` rejects:

- an unknown `constraint_type`, `verification_level`, `contract_role` (an empty string included) or `relationship`;
- a node `kind` other than `model`, `function` or `field`, because the checker starts paths only at `function` nodes and ends them only at `model` nodes;
- two node rows or two edge rows with one id;
- an edge whose source or target names no node;
- a node contract row whose `node_id` names no node;
- a per-edge contract row whose `edge_id` names no edge, names an edge without `source_override`, names an edge that starts at a node other than its `node_id`, or is not a postcondition, because each of these is ignored or misread today;
- a row with neither the value its kind reads nor a `dependent_expr`, and a row whose `dependent_expr` does not parse;
- the three malformed values rejected today.

A NULL `contract_role` stays a precondition. `range_min` stays an accepted `constraint_type`, though the schema does not list it.

**The open question from the plan, settled by measurement.** The plan left open whether a bound kind with no bound and no dependent expression is malformed or an absent constraint. It is malformed, for every kind. Across the 94 fixture databases, no row of any kind lacks both its value and a `dependent_expr`. The only rows without their value are 211 precision postconditions that carry a `dependent_expr`. A requirement without its bound passes every guarantee (`constraintImplies` is `True` when a bound is missing), so reading it as an absent constraint drops a requirement in silence. The extractor never writes one, so rejecting it costs no fixture. If the extractor ever has a reason to write such a row, it should write no row.

`cgv/README.md` and `cgv/CLAUDE.md` say what translation rejects. The SQL reads that produce the rows are still untested. TB-1.21 tests them.

## Affected users and systems
- Anyone who reads CGV's exit 0. A database with a row that translation cannot place now gives exit 2, not a verdict on the rest.
- `cgv/prover/ContractGraph/Translation.lean`, its callers in `ContractGraphTest`, and `cgv/tests/`. `Main.lean` keeps calling `readContractGraph`.
- TB-1.20, which proves `translateRows`. TB-1.21, which tests the reads.
- No protected surface. `Translation.lean` is not reached by any protected statement, and the manifest regenerates unchanged. Tier 2.

## Constraints
- No verdict of a database the extractor writes may change. `scripts/check-fixtures.sh` and `scripts/bench.py run --compare bench/baseline.json` show it.
- `translateRows` stays pure and total, so TB-1.20 can state "returns an error exactly when a row is malformed".
- Test graphs in `ContractGraphTest` are built through `translateRows`, by `graphOf`. A test whose rows are rejected panics, which `lake build` prints but passes, so its guards would run on an empty graph. `LEAN_ABORT_ON_PANIC=1 lake build` fails on such a panic, and passes on this change. CGV CI does not set it yet; TB-1.29 makes it.

## Open questions
None. The question the plan left to this intent is settled above.
