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

/-- Check choices consistency: source choices must be a subset of target choices. -/
def checkChoicesSubset (sourceChoices targetChoices : List String)
    (source target : Constraint) : CheckResult :=
  if sourceChoices.all (fun s => targetChoices.contains s) then
    .consistent
  else
    .inconsistent {
      severity := .error
      sourceConstraint := source
      targetConstraint := target
      path := []
      suggestion := s!"Source may produce choices not accepted by target. " ++
                    s!"Ensure source choices are a subset of target choices."
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
      match source.choicesList, target.choicesList with
      | some sc, some tc => checkChoicesSubset sc tc source target
      | _, _ => .consistent

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

/-- Check every matching constraint pair between source postconditions and
    target preconditions, returning one result per pair. Unlike `checkEdge`,
    this does not stop at the first inconsistency, so an edge that violates
    several constraint kinds (e.g. precision and nullability) reports each one. -/
def checkEdgeAll (source target : Node) : List CheckResult :=
  let pairs := source.postconditions.flatMap fun post =>
    target.preconditions.map fun pre => (post, pre)
  pairs.map fun (post, pre) => checkConstraintPair post pre

/-- `checkEdgeAll` applied to an edge's source and target. -/
def checkEdgeAllFull (edge : Edge) : List CheckResult :=
  checkEdgeAll edge.source edge.target

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
  | .choices =>
    match c.choicesList, d.choicesList with
    | some sc, some tc => ∀ x ∈ sc, x ∈ tc
    | _, _ => True

private theorem kind_bne_false {a b : ConstraintKind} (h : a = b) :
    (a != b) = false := by subst h; cases a <;> rfl

/-- Helper: foldl with an inconsistent accumulator stays inconsistent. -/
private theorem foldl_inconsistent_stays (tl : List (Constraint × Constraint))
    (d : DiagnosticInfo) :
    tl.foldl (fun (acc : CheckResult) (post, pre) =>
      match acc with
      | CheckResult.inconsistent _ => acc
      | CheckResult.consistent => checkConstraintPair post pre)
      (CheckResult.inconsistent d) = CheckResult.inconsistent d := by
  induction tl with
  | nil => rfl
  | cons hd tl ih => simp [List.foldl]; exact ih

set_option maxHeartbeats 800000 in
theorem pair_sound (c d : Constraint) (hk : c.kind = d.kind)
    (h : checkConstraintPair c d = .consistent) :
    constraintImplies c d := by
  unfold constraintImplies checkConstraintPair at *
  simp only [kind_bne_false hk, Bool.false_eq_true, ↓reduceIte] at h
  intro hk'
  rcases c with ⟨ck, csb, _, ctn, ccl, _, _, _⟩
  rcases d with ⟨dk, dsb, _, dtn, dcl, _, _, _⟩
  simp only at hk hk' ⊢ h
  subst hk
  cases ck <;> simp_all
  · -- precision
    cases csb <;> cases dsb <;> simp_all [checkStaticBounds]
  · -- nullability
    cases csb <;> cases dsb <;> simp_all
    rename_i sn tn
    unfold checkNullability at h
    split at h <;> simp_all
    rename_i h_impl
    by_cases hsn : sn = 0
    · exact Or.inl hsn
    · exact Or.inr (h_impl hsn)
  · -- type
    cases ctn <;> cases dtn <;> simp_all [checkTypeConsistency]
  · -- range
    cases csb <;> cases dsb <;> simp_all [checkStaticBounds]
  · -- length
    cases csb <;> cases dsb <;> simp_all [checkStaticBounds]
  · -- choices
    cases ccl <;> cases dcl <;> simp_all [checkChoicesSubset]

theorem foldl_consistent (pairs : List (Constraint × Constraint))
    (h : pairs.foldl (fun (acc : CheckResult) (post, pre) =>
      match acc with
      | CheckResult.inconsistent _ => acc
      | CheckResult.consistent => checkConstraintPair post pre) CheckResult.consistent = CheckResult.consistent) :
    ∀ p ∈ pairs, checkConstraintPair p.1 p.2 = CheckResult.consistent := by
  induction pairs with
  | nil => intro p hp; simp at hp
  | cons hd tl ih =>
    intro p hp
    simp [List.foldl] at h
    cases hcp : checkConstraintPair hd.1 hd.2 with
    | consistent =>
      rw [hcp] at h
      cases hp with
      | head => exact hcp
      | tail _ hmem => exact ih h p hmem
    | inconsistent di =>
      rw [hcp] at h
      rw [foldl_inconsistent_stays] at h
      exact absurd h (by simp)

theorem mem_pairs (source target : Node) (c d : Constraint)
    (hc : c ∈ source.postconditions) (hd : d ∈ target.preconditions) :
    (c, d) ∈ (source.postconditions.flatMap fun post =>
      target.preconditions.map fun pre => (post, pre)) := by
  grind

theorem checkEdge_sound (source target : Node) :
    checkEdge source target = .consistent →
    (∀ c ∈ source.postconditions, ∀ d ∈ target.preconditions,
      c.kind = d.kind →
      constraintImplies c d) := by
  intro h_check c hc d hd hk
  have h_all := foldl_consistent
    (source.postconditions.flatMap fun post => target.preconditions.map fun pre => (post, pre))
    h_check
  have h_mem := mem_pairs source target c d hc hd
  have h_pair := h_all (c, d) h_mem
  exact pair_sound c d hk h_pair

/-- Soundness of `checkEdgeAll`: if every per-pair result is consistent, the
    source postconditions imply the target preconditions. Same conclusion as
    `checkEdge_sound`. -/
theorem checkEdgeAll_sound (source target : Node)
    (h : ∀ r ∈ checkEdgeAll source target, r = .consistent) :
    ∀ c ∈ source.postconditions, ∀ d ∈ target.preconditions,
      c.kind = d.kind →
      constraintImplies c d := by
  intro c hc d hd hk
  have h_mem := mem_pairs source target c d hc hd
  have h_pair : checkConstraintPair c d = .consistent := by
    apply h
    unfold checkEdgeAll
    exact List.mem_map.mpr ⟨(c, d), h_mem, rfl⟩
  exact pair_sound c d hk h_pair

end ContractGraph
