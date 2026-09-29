> **Superseded (2026-09-29).** This document is historical. The `sorry` stubs it describes have all been proved, and there is no live `sorry` or `axiom` in `prover/ContractGraph/`. For the current state of the proofs, see the key theorems in `CLAUDE.md` (`checkEdge_sound`, `checkPath_sound`, `enumeratePaths_complete`, `closedStates_checkPath`, `runChecker_sound_all`). For the current measured behaviour of the tool, see `docs/evaluation/real-codebase-evaluation-2026-09.md`.

# Task

  Fill in sorry stubs in Lean soundness theorems for the contract-graph-verifier project.

  What Was Done

  1. Status doc written: docs/project/status-2026-03-26.md — comprehensive implementation status.
  2. Leanstral attempted: vibe --agent lean failed — it can read files but can't edit them, and its proof sketch still
  contained sorry.
  3. Manual proof work on Checker.lean — partially complete, hitting Lean 4 tactic issues:
    - Refactored checkEdge from foldl to recursive checkPairList for proof-friendliness (same behavior).
    - checkPairList_sound — proved successfully (foldl → all pairs consistent).
    - mem_pairs — proved successfully (cross-product membership).
    - pair_sound — the hard part. Current approach: split at h on the if-then-else, then cases c.kind <;> simp_all [...].
  Blocked on:
        - simp_all with checkNullability enters infinite recursion (simp loop on checkNullability.eq_1, Bool.and_eq_true,
  Bool.not_eq_eq_eq_not)
      - The first | ... | ... tactic combinator on line 173 throws "unknown tactic" — syntax issue
    - checkEdge_sound — written and correct, depends on the three helpers above.
  4. Composition.lean updated:
    - checkPath changed from partial def to def with termination_by path.length + decreasing_by simp_wf; omega — compiles
  successfully.
    - checkPath_sound theorem statement fixed (added (checkPath path).Forall (· = .consistent) precondition), left as sorry —
  proving this requires induction over path composition, much harder than checkEdge_sound.

  Current Build State

  - Checker.lean fails to build — the pair_sound proof needs fixing
  - Composition.lean builds (with sorry in checkPath_sound)
  - All other files build fine

  Key Technical Issues for Next Agent

  The pair_sound proof needs to show: checkConstraintPair c d = .consistent ∧ c.kind = d.kind → constraintImplies c d. The
  approach is:

  1. split at h on the if c.kind != d.kind — works, first branch closed by contradiction with kind_bne_false
  2. cases c.kind in the else branch — works, creates 6 subgoals
  3. For precision/length/range: need to case-split on c.staticBound/d.staticBound, unfold checkStaticBounds, extract sg ≤ tr
  from the if-then-else. Approach: split at h on the if, assumption for the true branch, contradiction for the false branch.
  4. For nullability: Bool↔Prop bridge problem. Need to go from ¬((sn != 0) && !(tn != 0) = true) to sn = 0 ∨ tn ≠ 0. The
  simp_all [checkNullability] loops — must exclude checkNullability.eq_1 from simp (use simp_all [checkNullability,
  -checkNullability.eq_1] or unfold manually).
  5. For type: need eq_of_beq to go from (st == tt) = true to st = tt.
  6. For choices: trivial.

  File Locations

  - prover/ContractGraph/Checker.lean — main file being edited (currently broken)
  - prover/ContractGraph/Composition.lean — updated, builds with sorry
  - prover/ContractGraph/Types.lean — defines all types (unchanged)
  - Build command: ~/.elan/bin/lake build ContractGraph (lake not on PATH, use full path)
