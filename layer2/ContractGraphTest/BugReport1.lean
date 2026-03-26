-- BugReport1.lean
-- Reproduces the field report Bug 1: precision mismatch

import ContractGraph.Types
import ContractGraph.Checker
import ContractGraph.Composition
import ContractGraph.Diagnostics

namespace ContractGraphTest.BugReport1

open ContractGraph

/-- split_energy function node:
    Body analysis infers precision(result) <= 6 from quantize(Decimal('0.000001')) -/
def splitEnergyNode : Node :=
  { id := 1
    name := "split_energy"
    kind := "function"
    preconditions := []
    postconditions := [
      { kind := .precision
        staticBound := some 6
        sourceFile := "billing/utils.py"
        sourceLine := 42
        verificationLevel := .extracted }
    ] }

/-- EnergyRecord.energy model node:
    DecimalField(max_digits=5, decimal_places=3) → precision <= 3 -/
def energyFieldNode : Node :=
  { id := 2
    name := "EnergyRecord.energy"
    kind := "model"
    preconditions := [
      { kind := .precision
        staticBound := some 3
        sourceFile := "billing/models.py"
        sourceLine := 15
        verificationLevel := .extracted }
    ]
    postconditions := [] }

/-- Edge: split_energy writes_to EnergyRecord.energy -/
def edge : Edge :=
  { source := splitEnergyNode
    target := energyFieldNode
    relationship := .writesTo }

/-- The contract graph for Bug 1. -/
def bug1Graph : ContractGraph :=
  { nodes := [splitEnergyNode, energyFieldNode]
    edges := [edge] }

/-- Test: checkEdge detects the precision mismatch. -/
#eval do
  let result := checkEdgeFull edge
  match result with
  | .inconsistent diag =>
    IO.println s!"BugReport1: PASS - Detected inconsistency"
    IO.println s!"  Suggestion: {diag.suggestion}"
  | .consistent =>
    IO.println "BugReport1: FAIL - Should have detected precision mismatch"

/-- Test: full pipeline detects Bug 1. -/
#eval do
  let output := runChecker bug1Graph
  IO.println s!"BugReport1 full pipeline:"
  IO.println s!"  Exit code: {output.exitCode}"
  IO.println s!"  Results: {output.results.length}"
  IO.println (outputToJson output)

end ContractGraphTest.BugReport1
