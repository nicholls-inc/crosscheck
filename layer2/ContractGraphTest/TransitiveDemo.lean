-- TransitiveDemo.lean
-- Demonstrates transitive inconsistency detection (Story 4.3)

import ContractGraph.Types
import ContractGraph.Checker
import ContractGraph.Composition
import ContractGraph.DependentExpr
import ContractGraph.Diagnostics
import ContractGraph.Main

namespace ContractGraphTest.TransitiveDemo

open ContractGraph

/-- compute_offpeak: body analysis → precision ≤ 4 -/
def computeOffpeakNode : Node :=
  { id := 1
    name := "compute_offpeak"
    kind := "function"
    preconditions := []
    postconditions := [
      { kind := .precision
        staticBound := some 4
        sourceFile := "billing/utils.py"
        sourceLine := 20
        verificationLevel := .extracted }
    ] }

/-- split_energy: dependent postcondition max(input_precision, 3) -/
def splitEnergyNode : Node :=
  { id := 2
    name := "split_energy"
    kind := "function"
    preconditions := [
      { kind := .precision
        staticBound := some 10
        sourceFile := "billing/utils.py"
        sourceLine := 42
        verificationLevel := .assumed }
    ]
    postconditions := [
      { kind := .precision
        depExpr := some (DepExpr.max (DepExpr.input "input_precision") (DepExpr.lit 3))
        sourceFile := "billing/utils.py"
        sourceLine := 42
        verificationLevel := .assumed }
    ] }

/-- EnergyRecord.energy: DecimalField(decimal_places=3) → precision <= 3 -/
def energyFieldNode : Node :=
  { id := 3
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

/-- Edge: compute_offpeak → split_energy -/
def edge1 : Edge :=
  { source := computeOffpeakNode
    target := splitEnergyNode
    relationship := .calls }

/-- Edge: split_energy → EnergyRecord.energy -/
def edge2 : Edge :=
  { source := splitEnergyNode
    target := energyFieldNode
    relationship := .writesTo }

/-- The transitive contract graph. -/
def transitiveGraph : ContractGraph :=
  { nodes := [computeOffpeakNode, splitEnergyNode, energyFieldNode]
    edges := [edge1, edge2] }

-- Test: pairwise check of compute_offpeak → split_energy is consistent.
#eval do
  let result := checkEdgeFull edge1
  match result with
  | .consistent =>
    IO.println "Pairwise A→B: PASS - Consistent (as expected)"
  | .inconsistent _ =>
    IO.println "Pairwise A→B: UNEXPECTED - Should be consistent"

-- Test: composition reveals transitive inconsistency.
#eval do
  -- Step 1: Compose compute_offpeak through split_energy
  let composed := composeContracts computeOffpeakNode splitEnergyNode
  IO.println s!"Composed postconditions: {composed.length}"
  for c in composed do
    IO.println s!"  kind={c.kind}, staticBound={c.staticBound}, depExpr={repr c.depExpr}"

  -- Step 2: Check composed guarantee against model
  let composedNode : Node := {
    id := splitEnergyNode.id
    name := splitEnergyNode.name
    kind := splitEnergyNode.kind
    preconditions := splitEnergyNode.preconditions
    postconditions := composed
  }
  let composedEdge : Edge := {
    source := composedNode
    target := energyFieldNode
    relationship := .writesTo
  }
  let result := checkEdgeFull composedEdge
  match result with
  | .inconsistent diag =>
    IO.println s!"Transitive check: PASS - Detected inconsistency"
    IO.println s!"  {diag.suggestion}"
  | .consistent =>
    IO.println "Transitive check: FAIL - Should have detected inconsistency"

-- Test: full pipeline detects transitive inconsistency.
#eval do
  let output := runChecker transitiveGraph
  IO.println s!"TransitiveDemo full pipeline:"
  IO.println s!"  Exit code: {output.exitCode}"
  IO.println s!"  Paths checked: {output.summary.pathsChecked}"
  IO.println s!"  Results: {output.results.length}"
  IO.println (outputToJson output)

end ContractGraphTest.TransitiveDemo
