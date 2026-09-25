-- Composition.lean

import ContractGraph.Types
import ContractGraph.DependentExpr
import ContractGraph.Checker
import ContractGraph.Diagnostics

namespace ContractGraph

/-- Compute the weakest verification level across a list of constraints. -/
def weakestIn (constraints : List Constraint) : VerificationLevel :=
  constraints.foldl (fun acc c => Min.min acc c.verificationLevel) .proved

/-- The weaker of two verification levels. -/
def weakerOf (a b : VerificationLevel) : VerificationLevel :=
  Min.min a b

/-- Path-level verification level: weakest link across all contracts. -/
def pathVerificationLevel (path : List Edge) : VerificationLevel :=
  path.foldl (fun acc edge =>
    weakerOf (weakerOf acc (weakestIn edge.source.postconditions))
             (weakestIn edge.target.preconditions)
  ) .proved

/-- Compose a source node's guarantees through a target node's
    dependent postconditions, producing the composed guarantee. -/
def composeContracts (source target : Node) : List Constraint :=
  target.postconditions.map fun postcon =>
    match postcon.depExpr with
    | none => postcon  -- static: passes through unchanged
    | some expr =>
      let inputs := source.postconditions.filterMap fun srcPost =>
        match srcPost.staticBound with
        | some bound => some (s!"input_{srcPost.kind}", bound)
        | none => none
      match evalDepExpr expr inputs with
      | some result =>
        { postcon with
          staticBound := some result
          depExpr := none
          verificationLevel :=
            weakerOf postcon.verificationLevel (weakestIn source.postconditions) }
      | none =>
        -- Evaluation failed: unresolved input binding or malformed expression.
        -- Preserve the unresolved postcondition and downgrade verification level.
        { postcon with verificationLevel := .extracted }

/-- Warnings for a hop's source postconditions whose dependent expression is
    still unresolved (no upstream binding: the path starts at this node, or the
    upstream postconditions lack the input kind). WARNING severity. -/
def collectUnresolvedWarnings (source target : Node) : List CheckResult :=
  source.postconditions.filterMap fun c =>
    match c.depExpr, c.staticBound with
    | some _, none =>
      some (.inconsistent (unresolvedDepWarning c source.name target.name))
    | _, _ => none

/-- Constraint kinds whose requirement passes vacuously when the source has no
    postcondition of that kind, and which therefore warn. -/
def warnsWhenMissing : ConstraintKind → Bool
  | .precision | .length | .range | .rangeMin | .nullability => true
  | .type | .choices => false

/-- Warnings for target preconditions (of a kind in `warnsWhenMissing`) that no
    source postcondition of the same kind addresses: the check for them passes
    vacuously. Applied on every hop, including the last one and single-edge
    paths; on later hops `source` is the composed intermediate node. -/
def collectMissingPostconditionWarnings (source target : Node) : List CheckResult :=
  let sourceKinds := source.postconditions.map (·.kind)
  target.preconditions.filterMap fun pre =>
    if warnsWhenMissing pre.kind && !sourceKinds.contains pre.kind then
      some (.inconsistent (missingPostconditionWarning pre source.name target.name))
    else none

/-- Results for one hop: one per constraint pair (`checkEdgeAllFull`, tagged
    with the hop), then the hop's warnings. -/
def checkHop (edge : Edge) : List CheckResult :=
  checkEdgeAllFull edge ++
    (collectUnresolvedWarnings edge.source edge.target ++
     collectMissingPostconditionWarnings edge.source edge.target)

/-- Check consistency across a multi-hop path by composing contracts at each
    step. Each hop contributes `checkHop` results, so several inconsistencies
    on the same edge are all reported, plus that hop's warnings.

    Composition goes through the NEXT edge's copy of the intermediate node
    (`nextEdge.source`): that copy carries the postconditions for this
    particular outgoing edge (a per-edge override), while `edge.target` carries
    the preconditions filtered for the incoming edge. The composed node keeps
    `edge.target`'s id, name, kind and preconditions.
    Non-partial: terminates by decreasing path length. -/
def checkPath (path : List Edge) : List CheckResult :=
  match path with
  | [] => [.consistent]
  | [edge] => checkHop edge
  | edge :: nextEdge :: remainingEdges =>
    let edgeResults := checkHop edge
    -- Compose upstream guarantees through the next edge's copy of the node
    let composedPostconditions := composeContracts edge.source nextEdge.source
    let composedNode : Node := {
      id := edge.target.id
      name := edge.target.name
      kind := edge.target.kind
      preconditions := edge.target.preconditions
      postconditions := composedPostconditions
    }
    -- Continue checking with composed node as source
    let updatedEdge : Edge := {
      source := composedNode
      target := nextEdge.target
      relationship := nextEdge.relationship
    }
    edgeResults ++ checkPath (updatedEdge :: remainingEdges)
termination_by path.length
decreasing_by simp_wf

/-- Enumerate all simple paths from function nodes to model nodes. -/
partial def findAllSimplePaths (edges : List Edge) (src tgt : Node)
    (visited : List Nat := []) : List (List Edge) :=
  if src.id == tgt.id then [[]]
  else if visited.contains src.id then []
  else
    let outEdges := edges.filter (fun e => e.source.id == src.id)
    outEdges.flatMap fun edge =>
      let subPaths := findAllSimplePaths edges edge.target tgt (src.id :: visited)
      subPaths.map fun subPath => edge :: subPath

/-- Edges that carry data and are checked: `writes_to` and `flows_to`.
    `calls` edges are structural only. -/
def checkedEdges (edges : List Edge) : List Edge :=
  edges.filter (fun e => e.relationship != .calls)

/-- Enumerate all paths from function nodes to model nodes, following only
    data edges (`calls` edges stay in the graph but are not followed). -/
def enumeratePaths (graph : ContractGraph) : List (List Edge) :=
  let modelNodes := graph.nodes.filter (·.kind == "model")
  let functionNodes := graph.nodes.filter (·.kind == "function")
  let edges := checkedEdges graph.edges
  functionNodes.flatMap fun src =>
    modelNodes.flatMap fun tgt =>
      findAllSimplePaths edges src tgt

/-- Check all paths and collect results with unresolved-dep warnings. -/
def checkAllPaths (graph : ContractGraph) : List (List Edge × List CheckResult) :=
  let paths := enumeratePaths graph
  paths.map fun path => (path, checkPath path)

/-- The composed guarantee from the first node in a path implies the
    assumptions of the last node (relative to the behavior model). -/
def composedGuaranteeImplies (source target : Node) : Prop :=
  ∀ c ∈ source.postconditions, ∀ d ∈ target.preconditions,
    c.kind = d.kind → constraintImplies c d

-- The original checkPath_sound theorem as stated below is FALSE for multi-hop paths.
-- Counterexample: Consider a path [edge1, edge2] where:
--   edge1.source.postconditions = [{precision, staticBound = 30}]
--   edge1.target.preconditions = [{precision, staticBound = 100}]   (30 ≤ 100 ✓)
--   edge1.target.postconditions = []
--   edge2.target.preconditions = [{precision, staticBound = 25}]
-- checkPath returns all consistent (each edge individually passes), but
-- composedGuaranteeImplies edge1.source edge2.target requires 30 ≤ 25, which is false.
-- The issue is that composedGuaranteeImplies relates the ORIGINAL source's postconditions
-- to the LAST target's preconditions, but checkPath only verifies each edge against
-- composed intermediate nodes — not the original source against the final target.
--
-- /--
-- SOUNDNESS THEOREM (path composition).
-- If checkPath returns all consistent, then the composed guarantee
-- from the first node implies the assumptions of the last node.
-- -/
-- theorem checkPath_sound (path : List Edge) (hne : path ≠ [])
--     (h : ∀ r ∈ checkPath path, r = CheckResult.consistent) :
--     composedGuaranteeImplies (path.head hne).source (path.getLast hne).target := by
--   sorry

/-- If every `checkHop` result is consistent, so is every per-pair result. -/
theorem checkHop_consistent (edge : Edge)
    (h : ∀ r ∈ checkHop edge, r = CheckResult.consistent) :
    ∀ r ∈ checkEdgeAllFull edge, r = CheckResult.consistent := by
  intro r hr; apply h; unfold checkHop; exact List.mem_append_left _ hr

/-- CORRECTED SOUNDNESS THEOREM (single-edge case).
    The original `checkPath_sound` was false for multi-hop paths because
    `composedGuaranteeImplies` relates the original source's postconditions to the
    last target's preconditions, but `checkPath` verifies each edge against composed
    intermediate nodes — not the original source against the final target directly.

    This corrected version proves the single-edge case, which IS sound: if checkPath
    on a single-edge path returns consistent, then the source's guarantees logically
    imply the target's assumptions. -/
theorem checkPath_sound_single (edge : Edge)
    (h : ∀ r ∈ checkPath [edge], r = CheckResult.consistent) :
    composedGuaranteeImplies edge.source edge.target := by
  unfold checkPath at h
  exact checkEdgeAll_sound edge.source edge.target (checkHop_consistent edge h)

/-- CORRECTED SOUNDNESS THEOREM (first-edge guarantee).
    For any non-empty path, if checkPath returns all consistent, then the first edge's
    source guarantees imply the first edge's target's assumptions. This is the strongest
    universally-true statement we can make from checkPath's all-consistent result. -/
theorem checkPath_sound_first_edge (path : List Edge) (hne : path ≠ [])
    (h : ∀ r ∈ checkPath path, r = CheckResult.consistent) :
    composedGuaranteeImplies (path.head hne).source (path.head hne).target := by
  match path, hne with
  | [edge], _ =>
    unfold checkPath at h
    exact checkEdgeAll_sound edge.source edge.target (checkHop_consistent edge h)
  | edge :: _ :: _, _ =>
    simp only [List.head_cons]
    have h1 : ∀ r ∈ checkHop edge, r = .consistent := by
      intro r hr; apply h; unfold checkPath; exact List.mem_append_left _ hr
    exact checkEdgeAll_sound edge.source edge.target (checkHop_consistent edge h1)

/-- Stepwise soundness predicate for multi-hop paths.
    Mirrors checkPath's recursive structure exactly: at each step, the current
    source's postconditions imply the current target's preconditions, and the
    remaining path is stepwise-sound with the composed intermediate node, whose
    postconditions are the next edge's copy of the node (`nextEdge.source`,
    carrying any per-edge override) composed with the upstream guarantees. -/
def stepwiseSound (path : List Edge) : Prop :=
  match path with
  | [] => True
  | [edge] => composedGuaranteeImplies edge.source edge.target
  | edge :: nextEdge :: remainingEdges =>
    composedGuaranteeImplies edge.source edge.target ∧
    let composedPostconditions := composeContracts edge.source nextEdge.source
    let composedNode : Node := {
      id := edge.target.id
      name := edge.target.name
      kind := edge.target.kind
      preconditions := edge.target.preconditions
      postconditions := composedPostconditions
    }
    let updatedEdge : Edge := {
      source := composedNode
      target := nextEdge.target
      relationship := nextEdge.relationship
    }
    stepwiseSound (updatedEdge :: remainingEdges)
termination_by path.length
decreasing_by simp_wf

/-- Helper: if all elements of as ++ bs are consistent, so are all elements
    of as and all elements of bs. -/
private theorem forall_consistent_of_append
    (as bs : List CheckResult)
    (h : ∀ r ∈ (as ++ bs), r = CheckResult.consistent) :
    (∀ r ∈ as, r = CheckResult.consistent) ∧ ∀ r ∈ bs, r = CheckResult.consistent := by
  constructor
  · intro r hr; apply h; exact List.mem_append_left _ hr
  · intro r hr; apply h; exact List.mem_append_right _ hr

set_option maxHeartbeats 400000 in
/--
SOUNDNESS THEOREM (multi-hop stepwise composition).
If checkPath returns all consistent, then stepwiseSound holds for the path:
at every step in the chain, the data flowing into that step (after any
transformations from prior steps) satisfies that step's requirements.

This is the correct replacement for the original checkPath_sound which was
proved FALSE by Aristotle: composedGuaranteeImplies between the original
source and the final target does not follow from checkPath's all-consistent
result. Instead, stepwiseSound captures what checkPath actually verifies:
each hop is sound with respect to the composed intermediate postconditions.

PROVIDED SOLUTION
Match on path, hne for three cases with termination_by path.length.

Case [] (empty path): contradiction with hne.

Case [edge] (single edge): Unfold checkPath to get checkHop edge, which is
  checkEdgeAllFull edge ++ warnings. checkHop_consistent gives every per-pair
  result consistent. Apply checkEdgeAll_sound.

Case edge :: nextEdge :: rest (multi-hop): checkPath produces
  checkHop edge ++ restPath where
  restPath = checkPath (updatedEdge :: rest), and updatedEdge's source is
  edge.target with postconditions composeContracts edge.source nextEdge.source.

  Step 1: Apply forall_consistent_of_append to h to extract:
    h1 : ∀ r ∈ checkHop edge, r = .consistent
    h2 : ∀ r ∈ checkPath (updatedEdge :: rest), r = .consistent

  Step 2 (first conjunct): Apply checkHop_consistent then checkEdgeAll_sound
    to h1 to get composedGuaranteeImplies edge.source edge.target.

  Step 3 (second conjunct): Apply checkPath_sound recursively on
    (updatedEdge :: rest) with h2. The termination obligation is
    (updatedEdge :: rest).length < (edge :: nextEdge :: rest).length,
    which is rest.length + 1 < rest.length + 2, discharged by omega.

  CRITICAL: stepwiseSound and checkPath construct composedNode and
  updatedEdge with identical field expressions — they unify definitionally.
  If unification fails, try unfold stepwiseSound.
-/
theorem checkPath_sound (path : List Edge) (hne : path ≠ [])
    (h : ∀ r ∈ checkPath path, r = CheckResult.consistent) :
    stepwiseSound path := by
  match path, hne with
  | [edge], _ =>
    unfold stepwiseSound
    unfold checkPath at h
    exact checkEdgeAll_sound edge.source edge.target (checkHop_consistent edge h)
  | edge :: nextEdge :: rest, _ =>
    unfold checkPath at h
    have ⟨h1, h2⟩ := forall_consistent_of_append
      (checkHop edge)
      (checkPath
        ({ source := { id := edge.target.id, name := edge.target.name, kind := edge.target.kind,
                       preconditions := edge.target.preconditions,
                       postconditions := composeContracts edge.source nextEdge.source },
           target := nextEdge.target, relationship := nextEdge.relationship } :: rest))
      h
    unfold stepwiseSound
    exact ⟨checkEdgeAll_sound _ _ (checkHop_consistent edge h1),
           checkPath_sound _ (List.cons_ne_nil _ _) h2⟩
termination_by path.length


/-! ## Soundness for error-free results

`checkPath_sound` needs every result to be `.consistent`, but every hop may
add warnings (`.inconsistent` with severity warning), so that hypothesis
rarely holds on a real run. The theorems below need only that no result is
an error, which is what exit code 0 reports. -/

/-- Unresolved-dependency warnings have severity warning. -/
theorem collectUnresolvedWarnings_severity (source target : Node) (r : CheckResult)
    (hr : r ∈ collectUnresolvedWarnings source target) :
    ∃ d, r = .inconsistent d ∧ d.severity = .warning := by
  unfold collectUnresolvedWarnings at hr
  obtain ⟨c, _, hc⟩ := List.mem_filterMap.mp hr
  split at hc
  · cases hc; exact ⟨_, rfl, rfl⟩
  · cases hc

/-- Missing-postcondition warnings have severity warning. -/
theorem collectMissingPostconditionWarnings_severity (source target : Node) (r : CheckResult)
    (hr : r ∈ collectMissingPostconditionWarnings source target) :
    ∃ d, r = .inconsistent d ∧ d.severity = .warning := by
  unfold collectMissingPostconditionWarnings at hr
  obtain ⟨c, _, hc⟩ := List.mem_filterMap.mp hr
  split at hc
  · cases hc; exact ⟨_, rfl, rfl⟩
  · cases hc

/-- Warnings are never errors. -/
theorem not_isError_of_warning (r : CheckResult)
    (h : ∃ d, r = .inconsistent d ∧ d.severity = .warning) : r.isError = false := by
  obtain ⟨d, rfl, hs⟩ := h
  simp [CheckResult.isError, hs]

/-- The results of `checkHop` beyond the per-pair ones are all warnings. -/
theorem checkHop_warnings_not_isError (edge : Edge) (r : CheckResult)
    (hr : r ∈ collectUnresolvedWarnings edge.source edge.target ++
              collectMissingPostconditionWarnings edge.source edge.target) :
    r.isError = false := by
  rcases List.mem_append.mp hr with h | h
  · exact not_isError_of_warning r (collectUnresolvedWarnings_severity _ _ r h)
  · exact not_isError_of_warning r (collectMissingPostconditionWarnings_severity _ _ r h)

/-- If no `checkHop` result is an error, no per-pair result is. -/
theorem checkHop_noErrors (edge : Edge)
    (h : ∀ r ∈ checkHop edge, r.isError = false) :
    ∀ r ∈ checkEdgeAllFull edge, r.isError = false := by
  intro r hr; apply h; unfold checkHop; exact List.mem_append_left _ hr

private theorem forall_noErrors_of_append
    (as bs : List CheckResult)
    (h : ∀ r ∈ (as ++ bs), r.isError = false) :
    (∀ r ∈ as, r.isError = false) ∧ ∀ r ∈ bs, r.isError = false :=
  ⟨fun r hr => h r (List.mem_append_left _ hr), fun r hr => h r (List.mem_append_right _ hr)⟩

set_option maxHeartbeats 400000 in
/-- SOUNDNESS THEOREM (error-free results). If no result of `checkPath` is an
    error (warnings allowed), `stepwiseSound` holds for the path. Same proof
    shape as `checkPath_sound`, with `checkEdgeAll_sound_noErrors` at each hop. -/
theorem checkPath_sound_noErrors (path : List Edge) (hne : path ≠ [])
    (h : ∀ r ∈ checkPath path, r.isError = false) :
    stepwiseSound path := by
  match path, hne with
  | [edge], _ =>
    unfold stepwiseSound
    unfold checkPath at h
    exact checkEdgeAll_sound_noErrors edge.source edge.target (checkHop_noErrors edge h)
  | edge :: nextEdge :: rest, _ =>
    unfold checkPath at h
    have ⟨h1, h2⟩ := forall_noErrors_of_append
      (checkHop edge)
      (checkPath
        ({ source := { id := edge.target.id, name := edge.target.name, kind := edge.target.kind,
                       preconditions := edge.target.preconditions,
                       postconditions := composeContracts edge.source nextEdge.source },
           target := nextEdge.target, relationship := nextEdge.relationship } :: rest))
      h
    unfold stepwiseSound
    exact ⟨checkEdgeAll_sound_noErrors _ _ (checkHop_noErrors edge h1),
           checkPath_sound_noErrors _ (List.cons_ne_nil _ _) h2⟩
termination_by path.length

/-- `checkPath_sound` is the special case where every result is consistent. -/
example (path : List Edge) (hne : path ≠ [])
    (h : ∀ r ∈ checkPath path, r = CheckResult.consistent) : stepwiseSound path :=
  checkPath_sound_noErrors path hne fun r hr => by rw [h r hr]; rfl

end ContractGraph
