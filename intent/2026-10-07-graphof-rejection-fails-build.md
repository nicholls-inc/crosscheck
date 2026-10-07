# Intent: Make a test graph with rejected rows fail the build

Task: TB-1.29. Governing roadmap item: TB-1. Issue: #16.

## Problem statement
The `ContractGraphTest` modules build their graphs with `graphOf` in `cgv/prover/ContractGraphTest/Translation.lean`. When `translateRows` rejects a test's rows, `graphOf` panics. A panic during elaboration prints a message and returns the default value, an empty graph, so `lake build` passes and the guards on that graph run on nothing. A guard such as "the checker reports no errors" passes on an empty graph.

Measured on `origin/main` at `30e0a8d` with Lean 4.28.0. A scratch module defines a graph whose two node rows share id 1, then guards that the checker reports no errors on it:

```
ℹ [54/54] Built ContractGraphTest.PanicRepro (385ms)
info: ContractGraphTest/PanicRepro.lean:11:0: PANIC at ContractGraphTest.Translation.graphOf ContractGraphTest.Translation:21:18: translateRows rejected test rows: two nodes have id 1
backtrace:
Build completed successfully (54 jobs).
```

`lake build` exits 0. CGV CI runs the same `lake build` with no other setting, so CI passes too. `cargo test`, the fixtures and the statement manifest do not build `ContractGraphTest` modules.

## Proposed outcome
`graphOf` takes a proof that `translateRows` accepts its rows, and the proof is filled in by `native_decide` when the call is elaborated. A call on rejected rows is an elaboration error, so `lake build` fails, on a developer's machine as in CI. With the default proof, a graph that `graphOf` returns is the graph `translateRows` built. A caller who passes `(accepted := sorry)` bypasses it with a warning, and CGV CI does not yet reject a `sorry` in `ContractGraphTest`; TB-1.32 shows it failing or names the property that blocks it. A `#guard_msgs` test in `ContractGraphTest/Translation.lean` checks that a call on rejected rows is an error, so the build fails if `graphOf` stops rejecting them.

The task row offered two fixes. Setting `LEAN_ABORT_ON_PANIC=1` for `lake build` in CGV CI would fail CI only, leave a local `lake build` green, and need a test outside the Lean build to show it works. Failing at elaboration fixes the cause, and its test runs in every `lake build`.

Other panics are not reached by this change. `[k]!` in `ContractGraph/StateSearch.lean` panics on an index out of bounds and returns a default, and a `#guard` that hits it could pass on that default. New row TB-1.31 sets `LEAN_ABORT_ON_PANIC=1` in CGV CI so that any panic fails it. It edits `.github/workflows/cgv-ci.yml`, a protected surface.

## Affected users and systems
- Anyone who writes a CGV Lean test with `graphOf`. Rejected rows now stop the build with the proposition `native_decide` found false.
- `cgv/prover/ContractGraphTest/Translation.lean`, where `graphOf` and the new `acceptedGraph` live.
- `Round3.lean`, `Round5.lean` and `Round6.lean`. A test graph with parameters cannot take the proof from `graphOf`, because `native_decide` cannot prove a proposition about free variables. Each of `microsGraph`, `choicesGraph`, `rangeGraph`, `callSiteGraph` and `originGraph` now translates its rows in a `*Rows` def and takes `accepted : (xRows ..).isOk := by native_decide`, so the proof runs at each concrete call. A new test graph with parameters follows the same shape. Every existing call is accepted, so no guard changes.
- The `ContractGraph` library, the checker binary and the statement manifest do not change.

## Constraints
- No protected surface changes. `native_decide` already appears in `ContractGraphTest` (`NoErrorsSoundness.lean`, `SoundnessDemo.lean`), and the library stays free of it.
- `lake build` stays green on `main`'s test graphs.

## Open questions
None.
