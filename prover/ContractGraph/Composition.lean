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
    upstream postconditions lack the input kind), one per target precondition
    of the same kind: the unresolved bound matters only where a requirement of
    its kind is checked, and the warning shows that requirement. WARNING
    severity. -/
def collectUnresolvedWarnings (source target : Node) : List CheckResult :=
  source.postconditions.flatMap fun c =>
    match c.depExpr, c.staticBound with
    | some _, none =>
      (target.preconditions.filter (·.kind == c.kind)).map fun pre =>
        .inconsistent (unresolvedDepWarning c source.name target.name pre)
    | _, _ => []

/-- Constraint kinds whose requirement passes vacuously when the source has no
    postcondition of that kind, and which therefore warn. -/
def warnsWhenMissing : ConstraintKind → Bool
  | .precision | .length | .range | .rangeMin | .nullability | .choices => true
  | .type => false

/-- Whether a requirement can reject a value at all: it has a bound (a choices
    list), and a nullability requirement is non-null (`staticBound = 0`); a
    nullable target accepts any value, so an unknown nullability is harmless. -/
def requirementMatters (pre : Constraint) : Bool :=
  match pre.kind with
  | .nullability => pre.staticBound == some 0
  | .choices => pre.choicesList.isSome
  | .type => pre.typeName.isSome
  | _ => pre.staticBound.isSome

/-- Warnings for target preconditions (of a kind in `warnsWhenMissing`, and
    that can reject a value: `requirementMatters`) that no source postcondition
    of the same kind addresses: the check for them passes vacuously. Applied on
    every hop, including the last one and single-edge paths; on later hops
    `source` is the composed intermediate node. -/
def collectMissingPostconditionWarnings (source target : Node) : List CheckResult :=
  let sourceKinds := source.postconditions.map (·.kind)
  target.preconditions.filterMap fun pre =>
    if warnsWhenMissing pre.kind && requirementMatters pre && !sourceKinds.contains pre.kind then
      some (.inconsistent (missingPostconditionWarning pre source.name target.name))
    else none

/-- Record a hop's locations on an inconsistent result: the hop source node's
    definition and the edge's site. Consistent results are unchanged. -/
def tagLocation (edge : Edge) : CheckResult → CheckResult
  | .consistent => .consistent
  | .inconsistent d => .inconsistent { d with
      hopSourceFile := edge.source.sourceFile, hopSourceLine := edge.source.sourceLine,
      siteFile := edge.siteFile, siteLine := edge.siteLine }

theorem tagLocation_eq_consistent (edge : Edge) (r : CheckResult) :
    tagLocation edge r = .consistent ↔ r = .consistent := by
  cases r <;> simp [tagLocation]

theorem tagLocation_isError (edge : Edge) (r : CheckResult) :
    (tagLocation edge r).isError = r.isError := by
  cases r <;> simp [tagLocation, CheckResult.isError]

/-- Results for one hop: one per constraint pair (`checkEdgeAllFull`, tagged
    with the hop), then the hop's warnings; every inconsistency also carries
    the hop's locations (`tagLocation`). -/
def checkHop (edge : Edge) : List CheckResult :=
  (checkEdgeAllFull edge ++
    (collectUnresolvedWarnings edge.source edge.target ++
     collectMissingPostconditionWarnings edge.source edge.target)).map (tagLocation edge)

/-- The edge that continues a path after `edge` along `nextEdge`, with the
    composed intermediate node as its source.

    Composition goes through the NEXT edge's copy of the intermediate node
    (`nextEdge.source`): that copy carries the postconditions for this
    particular outgoing edge (a per-edge override), while `edge.target` carries
    the preconditions filtered for the incoming edge. The composed node is
    `edge.target` (id, name, kind, preconditions, location) with the composed
    postconditions; the edge keeps `nextEdge`'s target, relationship and site. -/
def stepEdge (edge nextEdge : Edge) : Edge :=
  { source := { edge.target with
                postconditions := composeContracts edge.source nextEdge.source }
    target := nextEdge.target
    relationship := nextEdge.relationship
    siteFile := nextEdge.siteFile
    siteLine := nextEdge.siteLine }

/-- Check consistency across a multi-hop path by composing contracts at each
    step (`stepEdge`). Each hop contributes `checkHop` results, so several
    inconsistencies on the same edge are all reported, plus that hop's
    warnings. Non-partial: terminates by decreasing path length. -/
def checkPath (path : List Edge) : List CheckResult :=
  match path with
  | [] => [.consistent]
  | [edge] => checkHop edge
  | edge :: nextEdge :: remainingEdges =>
    checkHop edge ++ checkPath (stepEdge edge nextEdge :: remainingEdges)
termination_by path.length
decreasing_by simp_wf

/-- Enumerate all simple paths from `src` to `tgt` over `edges`, not revisiting
    the node ids in `visited`. Structural recursion on `fuel`, the maximum
    number of edges on a path. With `fuel > edges.length` (as `enumeratePathsNaive`
    uses) the fuel never runs out: every recursive call adds a distinct source
    id of some edge to `visited`, so the depth is at most `edges.length`, and
    the result equals that of the former unbounded (`partial`) search, in the
    same order. `enumeratePathsNaive_complete` proves every simple data path is
    found. -/
def findAllSimplePaths (edges : List Edge) (src tgt : Node)
    (visited : List Nat) : (fuel : Nat) → List (List Edge)
  | 0 => []
  | fuel + 1 =>
    if src.id == tgt.id then [[]]
    else if visited.contains src.id then []
    else
      let outEdges := edges.filter (fun e => e.source.id == src.id)
      outEdges.flatMap fun edge =>
        let subPaths := findAllSimplePaths edges edge.target tgt (src.id :: visited) fuel
        subPaths.map fun subPath => edge :: subPath

/-- Edges that carry data and are checked: `writes_to` and `flows_to`.
    `calls` edges are structural only. -/
def checkedEdges (edges : List Edge) : List Edge :=
  edges.filter (fun e => e.relationship != .calls)

/-- Enumerate all paths from function nodes to model nodes, following only
    data edges (`calls` edges stay in the graph but are not followed). -/
def enumeratePathsNaive (graph : ContractGraph) : List (List Edge) :=
  let modelNodes := graph.nodes.filter (·.kind == "model")
  let functionNodes := graph.nodes.filter (·.kind == "function")
  let edges := checkedEdges graph.edges
  functionNodes.flatMap fun src =>
    modelNodes.flatMap fun tgt =>
      findAllSimplePaths edges src tgt [] (edges.length + 1)

/-! ## Data paths and completeness of `enumeratePathsNaive` -/

/-- The edges of a path chain by node id, starting at node id `srcId`: the
    first edge leaves `srcId`, and each edge leaves the node the previous one
    enters. -/
def ChainFrom (srcId : Nat) : List Edge → Prop
  | [] => True
  | e :: rest => e.source.id = srcId ∧ ChainFrom e.target.id rest

/-- The node id a path starting at `srcId` ends at. -/
def endId (srcId : Nat) : List Edge → Nat
  | [] => srcId
  | e :: rest => endId e.target.id rest

/-- The node ids a path starting at `srcId` visits, in order, including both
    ends (`srcId` and `endId srcId path`). -/
def nodeIds (srcId : Nat) : List Edge → List Nat
  | [] => [srcId]
  | e :: rest => srcId :: nodeIds e.target.id rest

/-- A checked data path of `g`: non-empty, made of `g`'s non-`calls` edges,
    starting at a function node of `g` and ending at a model node of `g`
    (node identity is by id; edges carry copies of their endpoint nodes),
    chaining by node id, and simple (all visited node ids pairwise distinct). -/
def IsDataPath (g : ContractGraph) (p : List Edge) : Prop :=
  p ≠ [] ∧
  (∀ e ∈ p, e ∈ checkedEdges g.edges) ∧
  ∃ src ∈ g.nodes, ∃ tgt ∈ g.nodes,
    src.kind = "function" ∧ tgt.kind = "model" ∧
    ChainFrom src.id p ∧ endId src.id p = tgt.id ∧ (nodeIds src.id p).Nodup

theorem endId_mem_nodeIds (srcId : Nat) (p : List Edge) : endId srcId p ∈ nodeIds srcId p := by
  induction p generalizing srcId with
  | nil => simp [endId, nodeIds]
  | cons e rest ih => simp only [endId, nodeIds]; exact List.mem_cons_of_mem _ (ih _)

theorem nodeIds_eq (srcId : Nat) (p : List Edge) (h : ChainFrom srcId p) :
    nodeIds srcId p = p.map (·.source.id) ++ [endId srcId p] := by
  induction p generalizing srcId with
  | nil => rfl
  | cons e rest ih =>
    simp only [ChainFrom] at h
    simp only [nodeIds, endId, List.map_cons, List.cons_append, h.1]
    rw [ih _ h.2]

/-- Pigeonhole: a duplicate-free list contained in `m` is no longer than `m`. -/
theorem nodup_length_le (l m : List Nat) (hl : l.Nodup) (hsub : ∀ x ∈ l, x ∈ m) :
    l.length ≤ m.length := by
  induction l generalizing m with
  | nil => simp
  | cons a l ih =>
    have ⟨ha, hl'⟩ := List.nodup_cons.mp hl
    have ham : a ∈ m := hsub a (List.mem_cons_self)
    have h' := ih (m.erase a) hl' fun x hx => by
      have hxa : x ≠ a := fun hxa => ha (hxa ▸ hx)
      exact (List.mem_erase_of_ne hxa).mpr (hsub x (List.mem_cons_of_mem _ hx))
    rw [List.length_erase_of_mem ham] at h'
    have : 0 < m.length := List.length_pos_of_mem ham
    simp only [List.length_cons]; omega

/-- Every simple chain from `src` to `tgt` over `edges`, avoiding `visited`,
    with fewer edges than `fuel`, is found by `findAllSimplePaths`. -/
theorem mem_findAllSimplePaths (edges : List Edge) (tgt : Node) :
    ∀ (p : List Edge) (src : Node) (visited : List Nat) (fuel : Nat),
      (∀ e ∈ p, e ∈ edges) → ChainFrom src.id p → endId src.id p = tgt.id →
      (nodeIds src.id p).Nodup → (∀ x ∈ nodeIds src.id p, x ∉ visited) →
      p.length < fuel →
      p ∈ findAllSimplePaths edges src tgt visited fuel := by
  intro p
  induction p with
  | nil =>
    intro src visited fuel _ _ hend _ _ hlen
    match fuel, hlen with
    | f + 1, _ =>
      simp only [endId] at hend
      simp [findAllSimplePaths, hend]
  | cons e rest ih =>
    intro src visited fuel hedges hchain hend hnodup hvis hlen
    match fuel, hlen with
    | f + 1, hlen =>
      simp only [ChainFrom] at hchain
      simp only [endId] at hend
      simp only [nodeIds] at hnodup hvis
      have ⟨hnot, hnodup'⟩ := List.nodup_cons.mp hnodup
      have hne : src.id ≠ tgt.id := by
        intro h; apply hnot; rw [h, ← hend]; exact endId_mem_nodeIds _ _
      have hsv : src.id ∉ visited := hvis src.id List.mem_cons_self
      have hsub : rest ∈ findAllSimplePaths edges e.target tgt (src.id :: visited) f := by
        apply ih e.target (src.id :: visited) f
          (fun x hx => hedges x (List.mem_cons_of_mem _ hx)) hchain.2 hend hnodup'
        · intro x hx hmem
          rcases List.mem_cons.mp hmem with rfl | hv
          · exact hnot hx
          · exact hvis x (List.mem_cons_of_mem _ hx) hv
        · simp only [List.length_cons] at hlen; omega
      have hout : e ∈ edges.filter (fun e' => e'.source.id == src.id) :=
        List.mem_filter.mpr ⟨hedges e List.mem_cons_self, by simp [hchain.1]⟩
      simp only [findAllSimplePaths, beq_iff_eq, hne, if_false, List.contains_iff_mem, hsv]
      exact List.mem_flatMap.mpr ⟨e, hout, List.mem_map.mpr ⟨rest, hsub, rfl⟩⟩

/-- A data path has fewer edges than the graph has checked edges, plus one:
    its source ids are distinct ids of checked edges (pigeonhole). -/
theorem IsDataPath.length_lt {g : ContractGraph} {p : List Edge} (h : IsDataPath g p) :
    p.length < (checkedEdges g.edges).length + 1 := by
  obtain ⟨_, hedges, src, _, tgt, _, _, _, hchain, _, hnodup⟩ := h
  rw [nodeIds_eq _ _ hchain] at hnodup
  have hn := (List.nodup_append.mp hnodup).1
  have := nodup_length_le (p.map (·.source.id)) ((checkedEdges g.edges).map (·.source.id)) hn
    (fun x hx => by
      obtain ⟨e, he, rfl⟩ := List.mem_map.mp hx
      exact List.mem_map.mpr ⟨e, hedges e he, rfl⟩)
  simp only [List.length_map] at this
  omega

/-- COMPLETENESS of the naive enumerator (one search per (function, model)
    pair, no pruning): every checked data path of `g` is enumerated. The
    checker uses `enumeratePaths` (Search.lean), which is complete too
    (`enumeratePaths_complete`); this one is kept as a reference for tests. -/
theorem enumeratePathsNaive_complete (g : ContractGraph) (p : List Edge)
    (h : IsDataPath g p) : p ∈ enumeratePathsNaive g := by
  have hlen := h.length_lt
  obtain ⟨_, hedges, src, hsrc, tgt, htgt, hsk, htk, hchain, hend, hnodup⟩ := h
  unfold enumeratePathsNaive
  refine List.mem_flatMap.mpr ⟨src, List.mem_filter.mpr ⟨hsrc, by simp [hsk]⟩, ?_⟩
  refine List.mem_flatMap.mpr ⟨tgt, List.mem_filter.mpr ⟨htgt, by simp [htk]⟩, ?_⟩
  exact mem_findAllSimplePaths _ tgt p src [] _ hedges hchain hend hnodup
    (fun _ _ h => by cases h) hlen

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
  intro r hr
  rw [← tagLocation_eq_consistent edge]
  apply h; unfold checkHop
  exact List.mem_map.mpr ⟨r, List.mem_append_left _ hr, rfl⟩

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
    stepwiseSound (stepEdge edge nextEdge :: remainingEdges)
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
      (checkPath (stepEdge edge nextEdge :: rest))
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
  obtain ⟨c, _, hc⟩ := List.mem_flatMap.mp hr
  split at hc
  · obtain ⟨_, _, rfl⟩ := List.mem_map.mp hc; exact ⟨_, rfl, rfl⟩
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
  intro r hr
  rw [← tagLocation_isError edge]
  apply h; unfold checkHop
  exact List.mem_map.mpr ⟨r, List.mem_append_left _ hr, rfl⟩

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
      (checkPath (stepEdge edge nextEdge :: rest))
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
