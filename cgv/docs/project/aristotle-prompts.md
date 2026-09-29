# Aristotle Submission Prompts

Prompts for submitting `prover/` to [Aristotle](https://aristotle.harmonic.fun) to fill the 5 `sorry` stubs with verified proofs.

## Project submission

All prompts use:
```bash
aristotle submit "<PROMPT>" --project-dir ./prover --wait
```

## Prompt 1 — Primary (full project, all sorrys)

```
Fill in all the sorry stubs in this project. There are no Mathlib dependencies — use only core Lean 4 tactics.

Each sorry'd theorem has a PROVIDED SOLUTION docstring with a proof sketch. The key theorems in dependency order are:

1. pair_sound (Checker.lean) — the hardest proof. See its PROVIDED SOLUTION docstring for the detailed strategy. CRITICAL: do NOT use simp_all [checkNullability] as it causes an infinite loop via checkNullability.eq_1. Instead unfold checkNullability manually and case-split on Bool values.
2. foldl_consistent (Checker.lean) — standard list induction on the foldl accumulator.
3. mem_pairs (Checker.lean) — flatMap/map cross-product membership.
4. checkEdge_sound (Checker.lean) — assembles the three helpers above.
5. checkPath_sound (Composition.lean) — path induction using checkEdge_sound.

The kind_bne_false helper is already proved and available. ConstraintKind derives DecidableEq.
```

## Prompt 2 — Fallback (pair_sound only)

Use this if the full submission fails. Targets only the hardest piece.

```
Fill in the sorry in pair_sound only. This is a Lean 4 proof with no Mathlib.

The theorem shows that if checkConstraintPair c d returns .consistent and c.kind = d.kind, then constraintImplies c d holds. See the PROVIDED SOLUTION docstring for the full strategy.

Key warnings:
1. DO NOT use simp_all [checkNullability] — it causes an infinite loop via checkNullability.eq_1. Unfold checkNullability manually and case-split on the Bool values instead.
2. For the type case, use eq_of_beq to convert BEq equality to propositional equality.
3. kind_bne_false is already proved and available for bridging (a != b) = false from a = b.
4. ConstraintKind derives DecidableEq, so decide and simp can handle kind equality.
```

## Prompt 3 — Scaffolded (phased approach)

Explicitly phases the work by dependency order.

```
This project has 5 sorry stubs to fill, ordered by dependency. Each has a PROVIDED SOLUTION docstring. No Mathlib — core Lean 4 only.

PHASE 1 — No dependencies:
- pair_sound: The hardest proof. Follow the PROVIDED SOLUTION carefully. WARNING: simp_all [checkNullability] causes an infinite loop — unfold manually instead.
- foldl_consistent: Standard list induction on a short-circuit foldl.
- mem_pairs: Use List.mem_flatMap and List.mem_map for cross-product membership.

PHASE 2 — Depends on Phase 1:
- checkEdge_sound: Compose foldl_consistent, mem_pairs, and pair_sound.

PHASE 3 — Depends on Phase 2:
- checkPath_sound: Induction over path list, using checkEdge_sound for each edge.

ConstraintKind has DecidableEq. The kind_bne_false helper is already proved.
```

## Notes

- Aristotle reads `PROVIDED SOLUTION` from `/-- ... -/` doc comments above theorems
- Aristotle does NOT see `-- ...` comments inside proof blocks
- Aristotle skips `.olean` and `.lake/packages/` build artifacts
- The project builds cleanly with `lake build ContractGraph` (5 sorry warnings, no errors)
- If `checkPath_sound` cannot be proved, it is acceptable to leave as sorry — `checkEdge_sound` is the core soundness contribution
