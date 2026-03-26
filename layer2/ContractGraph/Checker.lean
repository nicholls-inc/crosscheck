-- Checker.lean

import ContractGraph.Types

namespace ContractGraph

/-- Check consistency between static bounds on a single edge.
    Source guarantees value ≤ S, target requires value ≤ T.
    Consistent iff S ≤ T. -/
def checkStaticBounds (sourceGuarantee targetRequirement : Int)
    (source target : Constraint) : CheckResult :=
  if sourceGuarantee ≤ targetRequirement then
    .consistent
  else
    .inconsistent {
      severity := .error
      sourceConstraint := source
      targetConstraint := target
      path := []
      suggestion := s!"Source guarantees ≤ {sourceGuarantee}, " ++
                    s!"target requires ≤ {targetRequirement}. " ++
                    s!"Either tighten the source or widen the target."
    }

/-- Check nullability consistency.
    null=True (source) + null=False (target) → inconsistent. -/
def checkNullability (sourceNullable targetNullable : Bool)
    (source target : Constraint) : CheckResult :=
  if sourceNullable && !targetNullable then
    .inconsistent {
      severity := .error
      sourceConstraint := source
      targetConstraint := target
      path := []
      suggestion := "Source may produce null values, but target requires non-null. " ++
                    "Add null check before writing, or set target field to null=True."
    }
  else
    .consistent

/-- Check type consistency. -/
def checkTypeConsistency (sourceType targetType : String)
    (source target : Constraint) : CheckResult :=
  if sourceType == targetType then
    .consistent
  else
    .inconsistent {
      severity := .error
      sourceConstraint := source
      targetConstraint := target
      path := []
      suggestion := s!"Source produces type '{sourceType}', target expects '{targetType}'."
    }

/-- Check a single constraint pair for consistency. -/
def checkConstraintPair (source target : Constraint) : CheckResult :=
  if source.kind != target.kind then
    .consistent  -- different constraint kinds don't interact
  else
    match source.kind with
    | .precision | .length | .range =>
      match source.staticBound, target.staticBound with
      | some sg, some tr => checkStaticBounds sg tr source target
      | _, _ => .consistent  -- missing bounds: cannot check
    | .nullability =>
      match source.staticBound, target.staticBound with
      | some sn, some tn =>
        checkNullability (sn != 0) (tn != 0) source target
      | _, _ => .consistent
    | .type =>
      match source.typeName, target.typeName with
      | some st, some tt => checkTypeConsistency st tt source target
      | _, _ => .consistent
    | .choices =>
      -- Choices consistency: source choices must be subset of target choices
      .consistent  -- simplified for PoC

/-- Check all matching constraint pairs between source postconditions
    and target preconditions. Returns first inconsistency found. -/
def checkEdge (source target : Node) : CheckResult :=
  let pairs := source.postconditions.flatMap fun post =>
    target.preconditions.map fun pre => (post, pre)
  pairs.foldl (fun acc (post, pre) =>
    match acc with
    | .inconsistent _ => acc
    | .consistent => checkConstraintPair post pre
  ) .consistent

/-- Check source postconditions against target preconditions or model constraints.
    For model nodes, preconditions ARE the model field constraints. -/
def checkEdgeFull (edge : Edge) : CheckResult :=
  checkEdge edge.source edge.target

/-- Predicate: a constraint implies another (source guarantee implies target requirement). -/
def constraintImplies (c d : Constraint) : Prop :=
  c.kind = d.kind →
  match c.kind with
  | .precision | .length | .range =>
    match c.staticBound, d.staticBound with
    | some sg, some tr => sg ≤ tr
    | _, _ => True
  | .nullability =>
    match c.staticBound, d.staticBound with
    | some sn, some tn => sn = 0 ∨ tn ≠ 0
    | _, _ => True
  | .type =>
    match c.typeName, d.typeName with
    | some st, some tt => st = tt
    | _, _ => True
  | .choices => True

private theorem kind_bne_false {a b : ConstraintKind} (h : a = b) :
    (a != b) = false := by subst h; cases a <;> rfl

/--
PROVIDED SOLUTION
Split on the `if source.kind != target.kind` guard in checkConstraintPair using
kind_bne_false hk. The true branch closes by contradiction (hk says kinds equal, guard
says not equal). In the else branch, subst hk and `cases c.kind` to get 6 subgoals:
- precision/length/range: case-split on c.staticBound and d.staticBound. When both are
  `some sg`/`some tr`, unfold checkStaticBounds at h, split on the `if sg ≤ tr` — the
  true branch gives the goal directly, the false branch gives .inconsistent = .consistent
  which is absurd.
- nullability: case-split on c.staticBound and d.staticBound. When both are some, DO NOT
  use `simp_all [checkNullability]` — it causes an infinite loop via checkNullability.eq_1.
  Instead unfold checkNullability at h and do casework on the Bool values of (sn != 0) and
  !(tn != 0). When sn = 0: Left; rfl. When tn ≠ 0: Right; exact the hypothesis.
- type: case-split on c.typeName and d.typeName. When both are some, unfold
  checkTypeConsistency at h. Use eq_of_beq to convert (st == tt) = true to st = tt.
- choices: the goal is True; use trivial.
-/
theorem pair_sound (c d : Constraint) (hk : c.kind = d.kind)
    (h : checkConstraintPair c d = .consistent) :
    constraintImplies c d := by
  sorry

/--
PROVIDED SOLUTION
Induction on the list. Base case: vacuously true (empty list). Inductive case: the foldl
only stays .consistent if the current element produces .consistent AND the rest of the foldl
produces .consistent. In the step, split on the match in the foldl body — if acc is
.inconsistent, foldl returns .inconsistent (contradiction with the hypothesis). If acc is
.consistent, then checkConstraintPair must return .consistent for the head, and the
induction hypothesis closes the tail.
-/
theorem foldl_consistent (pairs : List (Constraint × Constraint))
    (h : pairs.foldl (fun (acc : CheckResult) (post, pre) =>
      match acc with
      | CheckResult.inconsistent _ => acc
      | CheckResult.consistent => checkConstraintPair post pre) CheckResult.consistent = CheckResult.consistent) :
    ∀ p ∈ pairs, checkConstraintPair p.1 p.2 = CheckResult.consistent := by
  sorry

/--
PROVIDED SOLUTION
Unfold the flatMap/map definition. Use List.mem_flatMap to decompose membership in the
outer list, then List.mem_map to decompose membership in the inner list. The witnesses
are c (from hc) and d (from hd).
-/
theorem mem_pairs (source target : Node) (c d : Constraint)
    (hc : c ∈ source.postconditions) (hd : d ∈ target.preconditions) :
    (c, d) ∈ (source.postconditions.flatMap fun post =>
      target.preconditions.map fun pre => (post, pre)) := by
  sorry

/--
SOUNDNESS THEOREM (edge consistency).
If checkEdge returns consistent, then the source's guarantees
logically imply the target's assumptions.

PROVIDED SOLUTION
Unfold checkEdge. Apply foldl_consistent to the hypothesis to get that every pair in
the cross product has checkConstraintPair returning .consistent. Given c ∈ postconditions
and d ∈ preconditions, use mem_pairs to show (c, d) is in the cross product. Then apply
pair_sound with the kind equality hypothesis to conclude constraintImplies c d.
-/
theorem checkEdge_sound (source target : Node) :
    checkEdge source target = .consistent →
    (∀ c ∈ source.postconditions, ∀ d ∈ target.preconditions,
      c.kind = d.kind →
      constraintImplies c d) := by
  sorry

