-- Diagnostics.lean

import ContractGraph.Types

namespace ContractGraph

/-- Format a verification level for display. -/
def formatVerificationLevel : VerificationLevel → String
  | .proved => "PROVED"
  | .tested => "TESTED"
  | .extracted => "EXTRACTED"
  | .assumed => "ASSUMED"

/-- Format a constraint kind for display. -/
def formatConstraintKind : ConstraintKind → String
  | .precision => "precision"
  | .nullability => "nullability"
  | .type => "type"
  | .range => "range"
  | .length => "length"
  | .choices => "choices"

/-- Format a constraint's bound for display. -/
def formatBound (c : Constraint) : String :=
  match c.kind with
  | .precision | .length | .range =>
    match c.staticBound with
    | some b => s!"{formatConstraintKind c.kind} ≤ {b}"
    | none =>
      match c.depExpr with
      | some _ => s!"{formatConstraintKind c.kind} (dependent)"
      | none => s!"{formatConstraintKind c.kind} (unspecified)"
  | .nullability =>
    match c.staticBound with
    | some 0 => "non-null"
    | some _ => "nullable"
    | none => "nullability (unspecified)"
  | .type =>
    match c.typeName with
    | some t => s!"type = {t}"
    | none => "type (unspecified)"
  | .choices =>
    match c.choicesList with
    | some cs => s!"choices in [{", ".intercalate cs}]"
    | none => "choices (unspecified)"

/-- Build a result entry from a diagnostic. -/
def buildResultEntry (diag : DiagnosticInfo) (pathNames : List String)
    (verLevel : VerificationLevel) : ResultEntry :=
  { status := "inconsistent"
    severity := toString diag.severity
    source := {
      file := diag.sourceConstraint.sourceFile
      line := diag.sourceConstraint.sourceLine
      name := pathNames.head?.getD "unknown"
    }
    target := {
      file := diag.targetConstraint.sourceFile
      line := diag.targetConstraint.sourceLine
      name := pathNames.getLast?.getD "unknown"
    }
    path := pathNames
    sourceGuarantee := formatBound diag.sourceConstraint
    targetRequirement := formatBound diag.targetConstraint
    verificationLevel := formatVerificationLevel verLevel
    suggestion := diag.suggestion
  }

/-- Generate a warning for unresolved dependent expressions. -/
def unresolvedDepWarning (constraint : Constraint) (nodeName : String) : DiagnosticInfo :=
  { severity := .warning
    sourceConstraint := constraint
    targetConstraint := constraint
    path := [nodeName]
    suggestion := s!"Dependent expression on {nodeName} could not be resolved. " ++
                  s!"Check that upstream postconditions provide the required input bindings." }

end ContractGraph
