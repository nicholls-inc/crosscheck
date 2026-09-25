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

/-- Check consistency between static lower bounds on a single edge.
    Source guarantees value ≥ S, target requires value ≥ T.
    Consistent iff S ≥ T. -/
def checkLowerBounds (sourceGuarantee targetRequirement : Int)
    (source target : Constraint) : CheckResult :=
  if targetRequirement ≤ sourceGuarantee then
    .consistent
  else
    .inconsistent {
      severity := .error
      sourceConstraint := source
      targetConstraint := target
      path := []
      suggestion := s!"Source guarantees ≥ {sourceGuarantee}, " ++
                    s!"target requires ≥ {targetRequirement}. " ++
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
    | .rangeMin =>
      match source.staticBound, target.staticBound with
      | some sg, some tr => checkLowerBounds sg tr source target
      | _, _ => .consistent
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

/-- Record the hop (source and target node names) on an inconsistent result.
    Consistent results are unchanged. -/
def tagHop (hopSource hopTarget : String) : CheckResult → CheckResult
  | .consistent => .consistent
  | .inconsistent d => .inconsistent { d with hopSource := hopSource, hopTarget := hopTarget }

/-- Tagging never turns an inconsistent result into a consistent one, or back. -/
theorem tagHop_eq_consistent (s t : String) (r : CheckResult) :
    tagHop s t r = .consistent ↔ r = .consistent := by
  cases r <;> simp [tagHop]

/-- Check every matching constraint pair between source postconditions and
    target preconditions, returning one result per pair. Unlike `checkEdge`,
    this does not stop at the first inconsistency, so an edge that violates
    several constraint kinds (e.g. precision and nullability) reports each one.
    Each inconsistency is tagged (`tagHop`) with the source and target node
    names, so a report on a multi-hop path names the hop that failed. -/
def checkEdgeAll (source target : Node) : List CheckResult :=
  let pairs := source.postconditions.flatMap fun post =>
    target.preconditions.map fun pre => (post, pre)
  pairs.map fun (post, pre) => tagHop source.name target.name (checkConstraintPair post pre)

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
  | .rangeMin =>
    match c.staticBound, d.staticBound with
    | some sg, some tr => tr ≤ sg
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
  rcases c with ⟨ck, csb, _, ctn, ccl, _, _, _, _⟩
  rcases d with ⟨dk, dsb, _, dtn, dcl, _, _, _, _⟩
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
  · -- rangeMin
    cases csb <;> cases dsb <;> simp_all [checkLowerBounds]

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
    rw [← tagHop_eq_consistent source.name target.name]
    apply h
    unfold checkEdgeAll
    exact List.mem_map.mpr ⟨(c, d), h_mem, rfl⟩
  exact pair_sound c d hk h_pair


/-! ## Severity: every inconsistency found by pair checking is an error -/

/-- Every inconsistent result of `checkConstraintPair` has severity error. -/
theorem checkConstraintPair_severity (c d : Constraint) (di : DiagnosticInfo)
    (h : checkConstraintPair c d = .inconsistent di) : di.severity = .error := by
  unfold checkConstraintPair at h
  split at h
  · cases h
  · split at h <;> split at h <;>
      first
      | (cases h; done)
      | (simp only [checkStaticBounds, checkLowerBounds, checkNullability,
                    checkTypeConsistency, checkChoicesSubset] at h
         split at h <;> (cases h <;> rfl))

/-- `tagHop` keeps the severity of an inconsistency. -/
theorem tagHop_severity (s t : String) (r : CheckResult) (di : DiagnosticInfo)
    (h : tagHop s t r = .inconsistent di) :
    ∃ d, r = .inconsistent d ∧ di.severity = d.severity := by
  cases r with
  | consistent => cases h
  | inconsistent d =>
    simp only [tagHop, CheckResult.inconsistent.injEq] at h
    exact ⟨d, rfl, by rw [← h]⟩

/-- Every inconsistent result of `checkEdgeAll` (after hop tagging) is an error. -/
theorem checkEdgeAll_severity (source target : Node) (r : CheckResult)
    (hr : r ∈ checkEdgeAll source target) (di : DiagnosticInfo)
    (h : r = .inconsistent di) : di.severity = .error := by
  unfold checkEdgeAll at hr
  obtain ⟨⟨post, pre⟩, _, rfl⟩ := List.mem_map.mp hr
  obtain ⟨d, hd, hsev⟩ := tagHop_severity _ _ _ di h
  rw [hsev]
  exact checkConstraintPair_severity post pre d hd

/-- A result that can only be inconsistent with severity error, and is not an
    error, is consistent. -/
theorem eq_consistent_of_not_isError (r : CheckResult)
    (hsev : ∀ di, r = .inconsistent di → di.severity = .error)
    (h : r.isError = false) : r = .consistent := by
  cases r with
  | consistent => rfl
  | inconsistent di =>
    have hs := hsev di rfl
    simp [CheckResult.isError, hs] at h

/-- Soundness of `checkEdgeAll` from the weaker hypothesis that no result is
    an error. -/
theorem checkEdgeAll_sound_noErrors (source target : Node)
    (h : ∀ r ∈ checkEdgeAll source target, r.isError = false) :
    ∀ c ∈ source.postconditions, ∀ d ∈ target.preconditions,
      c.kind = d.kind →
      constraintImplies c d :=
  checkEdgeAll_sound source target fun r hr =>
    eq_consistent_of_not_isError r (checkEdgeAll_severity source target r hr) (h r hr)

end ContractGraph
