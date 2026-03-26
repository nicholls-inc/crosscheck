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

/-- Collect warnings for any postconditions with unresolved dependent expressions
    after composition. The spec mandates WARNING-severity diagnostics for these. -/
def collectUnresolvedWarnings (composed : List Constraint) (nodeName : String)
    : List CheckResult :=
  composed.filterMap fun c =>
    match c.depExpr, c.staticBound with
    | some _, none =>
      some (.inconsistent (unresolvedDepWarning c nodeName))
    | _, _ => none

/-- Check consistency across a multi-hop path by composing
    contracts at each step. Non-partial: terminates by decreasing path length. -/
def checkPath (path : List Edge) : List CheckResult :=
  match path with
  | [] => [.consistent]
  | [edge] => [checkEdgeFull edge]
  | edge :: nextEdge :: remainingEdges =>
    let edgeResult := checkEdgeFull edge
    -- Compose contracts through the edge
    let composedPostconditions := composeContracts edge.source edge.target
    let composedNode : Node := {
      id := edge.target.id
      name := edge.target.name
      kind := edge.target.kind
      preconditions := edge.target.preconditions
      postconditions := composedPostconditions
    }
    -- Emit warnings for any unresolved dependent expressions after composition
    let unresolvedWarnings := collectUnresolvedWarnings composedPostconditions edge.target.name
    -- Continue checking with composed node as source
    let updatedEdge : Edge := {
      source := composedNode
      target := nextEdge.target
      relationship := nextEdge.relationship
    }
    let restPath := checkPath (updatedEdge :: remainingEdges)
    edgeResult :: (unresolvedWarnings ++ restPath)
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

/-- Enumerate all paths from function nodes to model nodes. -/
def enumeratePaths (graph : ContractGraph) : List (List Edge) :=
  let modelNodes := graph.nodes.filter (·.kind == "model")
  let functionNodes := graph.nodes.filter (·.kind == "function")
  functionNodes.flatMap fun src =>
    modelNodes.flatMap fun tgt =>
      findAllSimplePaths graph.edges src tgt

/-- Check all paths and collect results with unresolved-dep warnings. -/
def checkAllPaths (graph : ContractGraph) : List (List Edge × List CheckResult) :=
  let paths := enumeratePaths graph
  paths.map fun path => (path, checkPath path)

/-- The composed guarantee from the first node in a path implies the
    assumptions of the last node (relative to the behavior model). -/
def composedGuaranteeImplies (source target : Node) : Prop :=
  ∀ c ∈ source.postconditions, ∀ d ∈ target.preconditions,
    c.kind = d.kind → constraintImplies c d

/--
SOUNDNESS THEOREM (path composition).
If checkPath returns all consistent, then the composed guarantee
from the first node implies the assumptions of the last node.

PROVIDED SOLUTION
Induction on path. Base case (single edge): unfold checkPath to get checkEdgeFull, then
apply checkEdge_sound. For the inductive case (edge :: nextEdge :: rest): the hypothesis
gives that edgeResult is .consistent (decompose List.Forall on the cons), so checkEdge_sound
applies to the first edge. The composed node's postconditions need to be shown to imply the
last node's preconditions via the induction hypothesis on the remaining path. The key insight
is that composedGuaranteeImplies is transitive through contract composition.
-/
theorem checkPath_sound (path : List Edge) (hne : path ≠ [])
    (h : ∀ r ∈ checkPath path, r = CheckResult.consistent) :
    composedGuaranteeImplies (path.head hne).source (path.getLast hne).target := by
  sorry

end ContractGraph
