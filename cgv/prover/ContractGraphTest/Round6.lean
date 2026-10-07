-- Round6.lean
-- Round-6 checker behaviour (docs/design/dataflow-v2.md, "Round 6",
-- "Checker"): `guarantee_at` of a bound composed from a dependent expression
-- is the input constraint whose value the result equals, else the
-- expression's own location. The origin is display-only.

import ContractGraph.Types
import ContractGraph.Checker
import ContractGraph.Composition
import ContractGraph.Translation
import ContractGraph.Main
import ContractGraphTest.Translation
import ContractGraphTest.Round5

namespace ContractGraphTest.Round6

open ContractGraph
open ContractGraphTest.Translation (graphOf acceptedGraph)
open ContractGraphTest.Round3 (contains errors warnings)

def againRow : ContractRow :=
  { nodeId := 4, constraintType := "precision", dependentExpr := some "max(input_precision, 2)",
    role := some "postcondition", sourceFile := "m.py", sourceLine := 20 }

/-- `five()` (5dp, m.py:9) → `keep(e)` (`max(input_precision, lit)`, m.py:15)
    → `S.e` (`pre` dp, models.py:3); then optionally → `again` (also
    `max(input_precision, 2)`, m.py:20) before `S.e`. -/
def originRows (lit pre : Int) (twice : Bool := false) : Except String ContractGraph :=
  translateRows
    (([(1, "five", "function"), (2, "keep", "function"), (3, "S.e", "model")] : List NodeRow) ++
      (if twice then [((4, "again", "function") : NodeRow)] else []))
    ([{ nodeId := 1, constraintType := "precision", decimalPlaces := some 5,
        role := some "postcondition", sourceFile := "m.py", sourceLine := 9 },
      { nodeId := 2, constraintType := "precision",
        dependentExpr := some s!"max(input_precision, {lit})",
        role := some "postcondition", sourceFile := "m.py", sourceLine := 15 },
      { nodeId := 3, constraintType := "precision", decimalPlaces := some pre,
        role := some "precondition", sourceFile := "models.py", sourceLine := 3 }] ++
     (if twice then [againRow] else []))
    ([{ id := 1, sourceId := 1, targetId := 2, relationship := "flows_to" }] ++
     (if twice then [{ id := 2, sourceId := 2, targetId := 4, relationship := "flows_to" },
                     { id := 3, sourceId := 4, targetId := 3, relationship := "writes_to" }]
      else [{ id := 2, sourceId := 2, targetId := 3, relationship := "writes_to" }]))

def originGraph (lit pre : Int) (twice : Bool := false)
    (accepted : (originRows lit pre twice).isOk := by native_decide) : ContractGraph :=
  acceptedGraph _ accepted

def errorAt (g : ContractGraph) : List (List String × String × String × Nat) :=
  (errors (runChecker g)).map fun r =>
    (r.path, r.sourceGuarantee, (guaranteeAt r).file, (guaranteeAt r).line)

-- max(5, 2) = 5 equals five()'s bound: guarantee_at is five()'s constraint
-- (m.py:9), not keep's expression (m.py:15).
#guard errorAt (originGraph 2 3) == [(["five", "keep", "S.e"], "precision ≤ 5", "m.py", 9)]
#guard contains (outputToJson (runChecker (originGraph 2 3)))
  "\"guarantee_at\": {\"file\": \"m.py\", \"line\": 9}"
-- max(5, 7) = 7 is the literal: the expression's own location (m.py:15).
#guard errorAt (originGraph 7 3) == [(["five", "keep", "S.e"], "precision ≤ 7", "m.py", 15)]
-- Through two dependent nodes the origin is carried: still five() (m.py:9).
#guard errorAt (originGraph 2 3 true)
  == [(["five", "keep", "again", "S.e"], "precision ≤ 5", "m.py", 9)]
-- A static guarantee: its own location.
#guard (errors (runChecker BugReport1.bug1Graph)).map (fun r => (guaranteeAt r).line)
  == (errors (runChecker BugReport1.bug1Graph)).map (·.guaranteeLine)

-- The composed constraint records the origin; the checks ignore it.
#guard ((composeContracts (originGraph 2 3).edges[0]!.source (originGraph 2 3).edges[1]!.source).map
    (fun c => (c.staticBound, c.originFile, c.originLine))) == [(some 5, "m.py", 9)]
example (c d : Constraint) (f : String) (l : Nat) :
    checkConstraintPair (c.withOrigin f l) d = .consistent ↔ checkConstraintPair c d = .consistent :=
  checkConstraintPair_withOrigin c d f l
example (c d : Constraint) (f : String) (l : Nat) :
    constraintImplies (c.withOrigin f l) d ↔ constraintImplies c d :=
  constraintImplies_withOrigin c d f l
-- Deduplication keys on the constraint's own location, not the origin: the
-- findings are the same as the path-based checker's.
#guard [originGraph 2 3, originGraph 7 3, originGraph 2 3 true, originGraph 2 5].all
  StateSearchTest.sameFindings
#guard (runChecker (originGraph 2 5)).exitCode == 0

end ContractGraphTest.Round6
