-- NumericTower.lean
-- The `type` check accepts an `int` where a `float` is required (PEP 484
-- numeric tower, `BehaviorModel.annotationAcceptsNumeric`), and nothing else
-- beyond equal type names. Task CG-1.7.

import ContractGraph.Types
import ContractGraph.Checker

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

end ContractGraphTest.NumericTower
