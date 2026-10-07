# Spec: `translateRows` translates every row or rejects the rows

Intent: `intent/2026-10-07-strict-translation.md`. Governing roadmap item: TB-1. Task: TB-1.12.

The executable form of each rule is a `#guard` in `cgv/prover/ContractGraphTest/Translation.lean` or a test in `cgv/tests/e2e_translation.rs`. A rule's ID names its guards in that file's sections.

- **ST-1. Purity.** `translateRows : List NodeRow → List ContractRow → List EdgeRow → Except String ContractGraph` does no IO. `readContractGraph` reads the three tables and calls it. An `.error msg` is exit 2, with `cannot translate the database: <msg>` on stderr.
- **ST-2. Accept.** Rows that break no rule below translate to one node per node row and one edge per edge row, as `buildGraph` built them. A NULL `contract_role` is a precondition. `range_min` is a `constraint_type`, read as a lower bound.
- **ST-3. Enum strings.** An unknown `constraint_type`, `verification_level`, `contract_role` (an empty string included) or `relationship` is an error. So is a node `kind` outside `model`, `function` and `field`.
- **ST-4. Ids.** Two node rows with one id, or two edge rows with one id, are an error.
- **ST-5. Edges.** An edge whose `source_node_id` or `target_node_id` names no node row is an error.
- **ST-6. Node contract rows.** A contract row with `edge_id` NULL whose `node_id` names no node row is an error.
- **ST-7. Per-edge contract rows.** A contract row with `edge_id = e` is an error when no edge row has id `e`, when edge `e` has `source_override = 0`, when edge `e` starts at a node other than the row's `node_id`, or when the row is not a postcondition.
- **ST-8. Values.** A row is an error when it has neither the value its kind reads nor a `dependent_expr`. The values are `param_decimal_places` (precision), `param_max_length` (length), `param_nullable` (nullability), `param_type_name` (type), `param_choices` (choices), any lower or upper bound (range), and a lower bound (`range_min`). A `dependent_expr` that `parseDepExpr` rejects is an error, as are the malformed `param_choices` and decimal bounds rejected before this change.
- **ST-9. No verdict moves.** Every fixture under `cgv/test_fixtures/` gives the verdict in its `expected.json`, and the bench corpus matches `cgv/bench/baseline.json`.

**Known gaps, not rules.**
- The SQL reads in `readNodes`, `readContracts` and `readEdges` are not checked. `readOptionalString` reads an empty string as NULL in every column but `contract_role`. TB-1.21 tests the reads against the rows the Rust writer inserted.
- `graphOf`, the test helper that builds a graph through `translateRows`, panics on rejected rows. `lake build` prints the panic and passes. TB-1.29 makes CGV CI fail on it.
- No theorem states these rules yet. TB-1.20 proves them.
