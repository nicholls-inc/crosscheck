-- NullableDemo.lean
-- Multi-constraint mismatch on one edge: precision and nullability.
-- A hand-built graph, not the nullable fixture's verdict: it puts both
-- inconsistencies on one write edge to show that checkEdgeAll reports every
-- constraint kind where checkEdge stops at the first. The pipeline on
-- test_fixtures/nullable/ reports the None at the return contract
-- (apply_discount -> apply_discount.<return>) instead, since apply_discount
-- returns None but never writes it (see test_fixtures/V2_FIXTURE_NOTES.md).

import ContractGraph.Types
import ContractGraph.Checker
import ContractGraph.Composition
import ContractGraph.Diagnostics
import ContractGraph.Main

namespace ContractGraphTest.NullableDemo

open ContractGraph

/-- apply_discount function node:
    Docstring gives precision(result) <= 4; body analysis finds a `return None` path. -/
def applyDiscountNode : Node :=
  { id := 1
    name := "apply_discount"
    kind := "function"
    preconditions := []
    postconditions := [
      { kind := .precision
        staticBound := some 4
        sourceFile := "utils.py"
        sourceLine := 7
        verificationLevel := .assumed },
      { kind := .nullability
        staticBound := some 1
        sourceFile := "utils.py"
        sourceLine := 7
        verificationLevel := .extracted }
    ] }

/-- Invoice.total model node:
    DecimalField(max_digits=10, decimal_places=2), null=False by default. -/
def totalFieldNode : Node :=
  { id := 2
    name := "Invoice.total"
    kind := "model"
    preconditions := [
      { kind := .precision
        staticBound := some 2
        sourceFile := "models.py"
        sourceLine := 5
        verificationLevel := .extracted },
      { kind := .nullability
        staticBound := some 0
        sourceFile := "models.py"
        sourceLine := 5
        verificationLevel := .extracted }
    ]
    postconditions := [] }

/-- Edge: apply_discount writes_to Invoice.total -/
def edge : Edge :=
  { source := applyDiscountNode
    target := totalFieldNode
    relationship := .writesTo }

def nullableGraph : ContractGraph :=
  { nodes := [applyDiscountNode, totalFieldNode]
    edges := [edge] }

/-- Constraint kinds of the inconsistent results in a result list. -/
def inconsistentKinds (results : List CheckResult) : List ConstraintKind :=
  results.filterMap fun r =>
    match r with
    | .inconsistent diag => some diag.sourceConstraint.kind
    | .consistent => none

-- checkEdge stops at the first inconsistency, so only precision is reported.
#guard inconsistentKinds [checkEdgeFull edge] == [.precision]

-- checkEdgeAll reports every inconsistent pair on the edge.
#guard inconsistentKinds (checkEdgeAllFull edge) == [.precision, .nullability]

-- runChecker on this graph reports both, with exit code 1.
#guard (runChecker nullableGraph).results.length == 2
#guard (runChecker nullableGraph).exitCode == 1

#eval do
  let output := runChecker nullableGraph
  IO.println s!"NullableDemo full pipeline:"
  IO.println s!"  Exit code: {output.exitCode}"
  IO.println s!"  Results: {output.results.length}"
  IO.println (outputToJson output)

end ContractGraphTest.NullableDemo
