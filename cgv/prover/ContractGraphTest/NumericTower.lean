-- NumericTower.lean
-- The `type` check accepts an `int` where a `float` is required (PEP 484
-- numeric tower, `BehaviorModel.annotationAcceptsNumeric`), and nothing else
-- beyond equal type names. Task CG-1.7.

import ContractGraph.Types
import ContractGraph.Checker
import ContractGraph.BehaviorModel
import ContractGraph.Composition

namespace ContractGraphTest.NumericTower

open ContractGraph

def ty (t : String) : Constraint :=
  { kind := .type, typeName := some t, sourceFile := "t.py", sourceLine := 1,
    verificationLevel := .extracted }

def isConsistent : CheckResult → Bool
  | .consistent => true
  | .inconsistent _ => false

#guard isConsistent (checkConstraintPair (ty "int") (ty "float"))
#guard isConsistent (checkConstraintPair (ty "float") (ty "float"))
#guard isConsistent (checkConstraintPair (ty "int") (ty "int"))
#guard !isConsistent (checkConstraintPair (ty "float") (ty "int"))
#guard !isConsistent (checkConstraintPair (ty "bool") (ty "float"))
#guard !isConsistent (checkConstraintPair (ty "Decimal") (ty "float"))
#guard !isConsistent (checkConstraintPair (ty "str") (ty "float"))
#guard !isConsistent (checkConstraintPair (ty "int") (ty "Decimal"))
#guard !isConsistent (checkConstraintPair (ty "int") (ty "complex"))

example : constraintImplies (ty "int") (ty "float") := by
  simp [constraintImplies, ty, typeAccepts]

example : ¬ constraintImplies (ty "float") (ty "int") := by
  simp [constraintImplies, ty, typeAccepts]

/-- The executable check and the trusted rule in `BehaviorModel.lean` agree on
    every pair of type names. If the two diverge, this stops building. An edit
    that changes both together still builds: that is for the governance note
    and the manifest hash of `typeAccepts` to catch. -/
theorem typeAccepts_iff_annotationAcceptsNumeric (s t : String) :
    typeAccepts s t = true ↔ BehaviorModel.annotationAcceptsNumeric s t := by
  simp [typeAccepts, BehaviorModel.annotationAcceptsNumeric]

/-- The protected `constraintImplies`, on two type constraints that both carry
    a name, says exactly what the trusted rule says. -/
theorem constraintImplies_type_iff (s t : String) :
    constraintImplies (ty s) (ty t) ↔ BehaviorModel.annotationAcceptsNumeric s t := by
  simp [constraintImplies, ty, typeAccepts, BehaviorModel.annotationAcceptsNumeric]

/-- The checker's `type` verdict is consistent exactly when the trusted rule
    accepts the pair. -/
theorem checkTypeConsistency_iff (s t : String) (a b : Constraint) :
    isConsistent (checkTypeConsistency s t a b) = true ↔
      BehaviorModel.annotationAcceptsNumeric s t := by
  rw [← typeAccepts_iff_annotationAcceptsNumeric]
  unfold checkTypeConsistency
  split <;> simp_all [isConsistent]

-- Multi-hop: a value widened from `int` to `float` at the middle node is then a
-- `float`, so it is still rejected where the last node requires an `int`.
def hopNode (id : Nat) (pre post : Option String) : Node :=
  { id := id, name := s!"n{id}", kind := "function"
    preconditions := (pre.map ty).toList
    postconditions := (post.map ty).toList }

def intSource   : Node := hopNode 1 none (some "int")
def floatMiddle : Node := hopNode 2 (some "float") (some "float")
def intSink     : Node := hopNode 3 (some "int") none

def errorCount (path : List Edge) : Nat :=
  ((checkPath path).filter fun r => !isConsistent r).length

theorem widenedIntIsFloatDownstream :
    errorCount [{ source := intSource, target := floatMiddle, relationship := .flowsTo },
                { source := floatMiddle, target := intSink, relationship := .flowsTo }] = 1 := by
  native_decide

end ContractGraphTest.NumericTower
