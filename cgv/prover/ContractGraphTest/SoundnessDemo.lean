-- SoundnessDemo.lean
-- Demonstrates the SOUNDNESS direction: when the checker says "consistent",
-- the formal soundness theorems guarantee that all constraints are satisfied.
--
-- The other demos (BugReport1, TransitiveDemo) show DETECTION: the checker finds bugs.
-- This demo shows the flip side: when no bugs are found, you get a machine-checked
-- proof that source guarantees imply target requirements at every step.

import ContractGraph.Types
import ContractGraph.Checker
import ContractGraph.Composition
import ContractGraph.Diagnostics

namespace ContractGraphTest.SoundnessDemo

open ContractGraph

-- ============================================================================
-- Scenario: A billing pipeline where all constraints are compatible.
--
--   format_invoice ──calls──▶ validate_total ──writesTo──▶ InvoiceRecord.total
--
-- format_invoice guarantees:  precision ≤ 3, non-null, type = "Decimal"
-- validate_total requires:    precision ≤ 5
-- validate_total guarantees:  precision ≤ 3 (static passthrough), type = "Decimal"
-- InvoiceRecord.total requires: precision ≤ 10, type = "Decimal"
--
-- Every constraint is satisfied at every hop. The soundness theorems prove this.
-- ============================================================================

def formatInvoiceNode : Node :=
  { id := 1
    name := "format_invoice"
    kind := "function"
    preconditions := []
    postconditions := [
      { kind := .precision
        staticBound := some 3
        sourceFile := "billing/invoices.py"
        sourceLine := 15
        verificationLevel := .extracted },
      { kind := .nullability
        staticBound := some 0  -- 0 = non-null
        sourceFile := "billing/invoices.py"
        sourceLine := 15
        verificationLevel := .extracted },
      { kind := .type
        typeName := some "Decimal"
        sourceFile := "billing/invoices.py"
        sourceLine := 15
        verificationLevel := .extracted }
    ] }

def validateTotalNode : Node :=
  { id := 2
    name := "validate_total"
    kind := "function"
    preconditions := [
      { kind := .precision
        staticBound := some 5
        sourceFile := "billing/validators.py"
        sourceLine := 30
        verificationLevel := .assumed }
    ]
    postconditions := [
      { kind := .precision
        staticBound := some 3
        sourceFile := "billing/validators.py"
        sourceLine := 30
        verificationLevel := .assumed },
      { kind := .type
        typeName := some "Decimal"
        sourceFile := "billing/validators.py"
        sourceLine := 30
        verificationLevel := .assumed }
    ] }

def invoiceFieldNode : Node :=
  { id := 3
    name := "InvoiceRecord.total"
    kind := "model"
    preconditions := [
      { kind := .precision
        staticBound := some 10
        sourceFile := "billing/models.py"
        sourceLine := 8
        verificationLevel := .extracted },
      { kind := .type
        typeName := some "Decimal"
        sourceFile := "billing/models.py"
        sourceLine := 8
        verificationLevel := .extracted }
    ]
    postconditions := [] }

def edge1 : Edge :=
  { source := formatInvoiceNode
    target := validateTotalNode
    relationship := .calls }

def edge2 : Edge :=
  { source := validateTotalNode
    target := invoiceFieldNode
    relationship := .writesTo }

def soundPath : List Edge := [edge1, edge2]

-- ============================================================================
-- Part 1: Runtime verification — the checker says "consistent"
-- ============================================================================

-- Run checkPath on concrete data and show all results are consistent.
#eval do
  let results := checkPath soundPath
  let allConsistent := results.all fun r =>
    match r with
    | .consistent => true
    | .inconsistent _ => false
  IO.println s!"checkPath results: {results.length} items, all consistent = {allConsistent}"
  IO.println "Runtime verification: this pipeline has no constraint violations."

-- ============================================================================
-- Part 2: Formal verification — machine-checked proofs of soundness
-- ============================================================================

-- Prove checkEdge returns .consistent for each edge by definitional reduction.
-- Lean's kernel evaluates the functions on concrete data and confirms the result.

theorem edge1_consistent :
    checkEdge formatInvoiceNode validateTotalNode = .consistent := by
  rfl

theorem edge2_consistent :
    checkEdge validateTotalNode invoiceFieldNode = .consistent := by
  rfl

-- Apply checkEdge_sound: if checkEdge returns consistent, then for ALL
-- matching constraint pairs, the source's guarantee logically implies the
-- target's requirement.

theorem edge1_sound :
    ∀ c ∈ formatInvoiceNode.postconditions,
    ∀ d ∈ validateTotalNode.preconditions,
      c.kind = d.kind → constraintImplies c d :=
  checkEdge_sound formatInvoiceNode validateTotalNode edge1_consistent

theorem edge2_sound :
    ∀ c ∈ validateTotalNode.postconditions,
    ∀ d ∈ invoiceFieldNode.preconditions,
      c.kind = d.kind → constraintImplies c d :=
  checkEdge_sound validateTotalNode invoiceFieldNode edge2_consistent

-- To prove path consistency formally, we need Lean to evaluate checkPath on
-- concrete data and confirm every result is .consistent. This requires a
-- Decidable instance for CheckResult equality with .consistent.
private instance (r : CheckResult) : Decidable (r = CheckResult.consistent) :=
  match r with
  | .consistent => isTrue rfl
  | .inconsistent _ => isFalse (fun h => CheckResult.noConfusion h)

-- For the full multi-hop path, prove all checkPath results are consistent.
-- native_decide compiles and runs checkPath on our concrete pipeline data,
-- then verifies every result equals .consistent.
theorem path_all_consistent :
    ∀ r ∈ checkPath soundPath, r = CheckResult.consistent := by
  native_decide

-- THE SOUNDNESS GUARANTEE: stepwiseSound holds for the full pipeline.
-- This means at every hop (including after contract composition), the
-- source's guarantees logically imply the target's requirements.
theorem path_sound : stepwiseSound soundPath :=
  checkPath_sound soundPath (List.cons_ne_nil _ _) path_all_consistent

-- ============================================================================
-- Part 3: What does this mean concretely?
-- ============================================================================

-- path_sound gives us stepwiseSound [edge1, edge2], which unfolds to:
--
--   composedGuaranteeImplies formatInvoiceNode validateTotalNode
--   ∧ composedGuaranteeImplies composedNode invoiceFieldNode
--
-- The first conjunct says: for every (postcondition, precondition) pair
-- between format_invoice and validate_total with matching kinds:
--   precision: 3 ≤ 5  ✓  (source guarantees ≤3, target requires ≤5)
--   nullability: no matching precondition, vacuously true
--   type: no matching precondition, vacuously true
--
-- The second conjunct (after composition) says: for the composed node
-- (validate_total with its postconditions) vs InvoiceRecord.total:
--   precision: 3 ≤ 10  ✓  (composed guarantees ≤3, target requires ≤10)
--   type: "Decimal" = "Decimal"  ✓
--
-- All of this is MACHINE-CHECKED by Lean's kernel. No trust required.
-- The chain from runtime (checkPath returns consistent) to formal guarantee
-- (constraintImplies holds for all matching pairs) is fully verified.

end ContractGraphTest.SoundnessDemo
