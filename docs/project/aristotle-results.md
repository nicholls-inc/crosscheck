# Aristotle Verification Results

Submission date: 2026-03-26
Project ID: `3ef32fda-98ff-4840-8b24-bb5c47e29ec3`
Duration: ~2h 48m (queued ~4min, in-progress ~2h 44m)
Prompt used: Prompt 1 — Primary (full project, all sorrys)
Lean toolchain: 4.28.0

## Summary

Aristotle was submitted the full `prover/` project with 5 `sorry` stubs to fill.
It **proved 4 of 5 theorems** and **discovered the 5th is false**, providing a
counterexample and two corrected alternatives. The project now builds with
**zero `sorry` warnings and zero errors**.

## Results by theorem

### 1. `pair_sound` (Checker.lean) — Proved

The hardest proof. Shows that if `checkConstraintPair c d` returns `.consistent`
and `c.kind = d.kind`, then `constraintImplies c d` holds.

Aristotle's approach:
- Unfolds `constraintImplies` and `checkConstraintPair`
- Uses `kind_bne_false` to simplify the guard, then destructs both constraints with `rcases`
- Cases on `ConstraintKind` (6 branches: precision, nullability, type, range, length, choices)
- For precision/range/length: case-splits on `staticBound` options, unfolds `checkStaticBounds`
- For nullability: manually unfolds `checkNullability` (avoiding the `simp_all` infinite loop),
  then uses `by_cases` on `sn = 0` to produce `Or.inl`/`Or.inr`
- For type: case-splits on `typeName` options, unfolds `checkTypeConsistency`
- Required `maxHeartbeats 800000`

Also introduced a helper lemma `foldl_inconsistent_stays` proving that `foldl` with an
`.inconsistent` accumulator stays `.inconsistent`.

### 2. `foldl_consistent` (Checker.lean) — Proved

Shows that if the short-circuit foldl over constraint pairs returns `.consistent`,
then every individual pair returns `.consistent`.

Aristotle's approach:
- List induction with `induction pairs`
- Base case: `simp at hp` (vacuously true)
- Inductive case: cases on `checkConstraintPair hd.1 hd.2`:
  - If `.consistent`: rewrite and use `ih` for tail membership
  - If `.inconsistent`: apply `foldl_inconsistent_stays` to derive contradiction

### 3. `mem_pairs` (Checker.lean) — Proved

Shows that `(c, d)` is in the `flatMap`/`map` cross-product when `c` is in
postconditions and `d` is in preconditions.

Aristotle's approach: single tactic — `grind`.

### 4. `checkEdge_sound` (Checker.lean) — Proved

The core soundness theorem for edge consistency. Shows that if `checkEdge` returns
`.consistent`, then for all matching constraint pairs, the source implies the target.

Aristotle's approach:
- Applies `foldl_consistent` to get that every pair in the cross product passes
- Uses `mem_pairs` to show the specific `(c, d)` pair is in the cross product
- Applies `pair_sound` with the kind equality hypothesis

### 5. `checkPath_sound` (Composition.lean) — FALSE

Aristotle discovered the original theorem statement is **false for multi-hop paths**
and cannot be proved.

#### Counterexample

Consider a path `[edge1, edge2]` where:

| Component | Kind | staticBound |
|---|---|---|
| edge1.source.postconditions | precision | 30 |
| edge1.target.preconditions | precision | 100 |
| edge1.target.postconditions | (empty) | — |
| edge2.target.preconditions | precision | 25 |

- `checkPath` returns all `.consistent` — each edge individually passes (30 <= 100, and
  the composed node has empty postconditions so the second edge trivially passes)
- But `composedGuaranteeImplies edge1.source edge2.target` requires 30 <= 25, which is false

#### Root cause

`composedGuaranteeImplies` relates the **original** source's postconditions to the
**last** target's preconditions. But `checkPath` verifies each edge against **composed
intermediate nodes** — not the original source against the final target directly. When
intermediate postconditions are empty or weaker than the original source's guarantees,
the chain of implications breaks.

#### Corrected theorems (proved)

Aristotle provided two corrected alternatives that ARE provable:

1. **`checkPath_sound_single`**: For a single-edge path, if `checkPath` returns all
   consistent, then `composedGuaranteeImplies edge.source edge.target` holds.

2. **`checkPath_sound_first_edge`**: For any non-empty path, if `checkPath` returns all
   consistent, then the first edge's source guarantees imply the first edge's target's
   assumptions. This is the strongest universally-true statement derivable from
   `checkPath`'s all-consistent result.

## Multi-hop soundness theorem (second submission)

Submission date: 2026-03-28
Project ID: `fbaa3d79-53df-4d90-880f-50115c89a305`
Duration: ~25 minutes
Prompt used: Targeted prompt for single sorry in `checkPath_sound`

### 6. `checkPath_sound` (Composition.lean) — Proved

The correct multi-hop soundness theorem. Introduces `stepwiseSound`, a recursive
predicate that mirrors `checkPath`'s exact structure: at each hop, the current
source's postconditions (after composition from prior steps) imply the current
target's preconditions.

In plain terms: if every hop in a data pipeline passes verification, then at every
step the data flowing into that step (after any transformations from prior steps)
satisfies that step's requirements. The whole chain is verifiably correct because
every link is verified, accounting for how data transforms as it flows through.

Aristotle's approach:
- Match on `path, hne` for single-edge and multi-hop cases
- Single edge: unfold `stepwiseSound` and `checkPath`, apply `checkEdge_sound`
- Multi-hop: unfold `checkPath`, apply `forall_consistent_of_cons_append` to decompose
  the result list, then `And.intro` with `checkEdge_sound` for the first conjunct and
  a recursive call to `checkPath_sound` for the second
- Termination by `path.length` (decreasing because the recursive path is one edge shorter)
- Required `maxHeartbeats 400000`

Also introduced:
- `stepwiseSound` predicate — mirrors `checkPath` with identical field construction
  for `composedNode` and `updatedEdge` so they unify definitionally
- `forall_consistent_of_cons_append` helper — extracts membership facts from
  `a :: (bs ++ cs)` structure that `checkPath` produces

## Current status

**All soundness theorems are now proved.** The project builds with zero `sorry`
warnings and zero errors. The full theorem inventory:

| Theorem | File | Status |
|---|---|---|
| `pair_sound` | Checker.lean | Proved |
| `foldl_consistent` | Checker.lean | Proved |
| `mem_pairs` | Checker.lean | Proved |
| `checkEdge_sound` | Checker.lean | Proved |
| `checkPath_sound_single` | Composition.lean | Proved |
| `checkPath_sound_first_edge` | Composition.lean | Proved |
| `stepwiseSound` | Composition.lean | Definition |
| `checkPath_sound` | Composition.lean | Proved |

## Files modified

- `prover/ContractGraph/Checker.lean` — 4 `sorry` stubs replaced with verified proofs
- `prover/ContractGraph/Composition.lean` — original `checkPath_sound` commented out
  with counterexample documentation; `stepwiseSound` predicate, helper lemma, and
  correct `checkPath_sound` theorem added with verified proofs
