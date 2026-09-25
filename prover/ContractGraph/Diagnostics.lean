-- Diagnostics.lean

import ContractGraph.Types

namespace ContractGraph

/-- A constraint's display origin: `originFile/originLine` when set, else
    its own location. -/
def Constraint.originOrOwn (c : Constraint) : String × Nat :=
  if c.originFile.isEmpty then (c.sourceFile, c.sourceLine) else (c.originFile, c.originLine)

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
  | .rangeMin => "range_min"

/-- Format a constraint's bound for display. -/
def formatBound (c : Constraint) : String :=
  match c.kind with
  | .precision | .length | .range =>
    match c.staticBound with
    | some b => s!"{formatConstraintKind c.kind} ≤ {formatBoundValue c.kind b c.scale}"
    | none =>
      match c.depExpr with
      | some _ => s!"{formatConstraintKind c.kind} (dependent)"
      | none => s!"{formatConstraintKind c.kind} (unspecified)"
  | .rangeMin =>
    match c.staticBound with
    | some b => s!"range ≥ {formatScaled b c.scale}"
    | none =>
      match c.depExpr with
      | some _ => "range_min (dependent)"
      | none => "range_min (unspecified)"
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

/-- Build a result entry from a diagnostic. `head` is the path head node
    (`none`: unknown, then the source constraint's location is used).
    - `source`: for an error, the path head (name and definition location);
      for a warning, the hop source node (the head when untagged).
    - `target.name` is the hop target (the node whose precondition failed),
      falling back to the path's last node for an untagged diagnostic;
      `target.file/line` is the failing requirement's location.
    - `hop` is `[hop source, hop target]`; `site` is the hop edge's site. -/
def buildResultEntry (diag : DiagnosticInfo) (pathNames : List String)
    (verLevel : VerificationLevel) (head : Option Node := none) : ResultEntry :=
  let headName := pathNames.head?.getD "unknown"
  let lastName := pathNames.getLast?.getD "unknown"
  let hopSource := if diag.hopSource.isEmpty then headName else diag.hopSource
  let hopTarget := if diag.hopTarget.isEmpty then lastName else diag.hopTarget
  -- A node without a recorded location (graphs built in Lean tests) falls
  -- back to the source constraint's location.
  let constraintLoc (name : String) : SourceLocation :=
    { file := diag.sourceConstraint.sourceFile, line := diag.sourceConstraint.sourceLine,
      name := name }
  let headLoc : SourceLocation :=
    match head with
    | some n =>
      if n.sourceFile.isEmpty then constraintLoc headName
      else { file := n.sourceFile, line := n.sourceLine, name := headName }
    | none => constraintLoc headName
  let source : SourceLocation :=
    match diag.severity with
    | .error => headLoc
    | .warning =>
      if diag.hopSource.isEmpty then headLoc
      else if diag.hopSourceFile.isEmpty then constraintLoc hopSource
      else { file := diag.hopSourceFile, line := diag.hopSourceLine, name := hopSource }
  { status := "inconsistent"
    severity := toString diag.severity
    source := source
    target := {
      file := diag.targetConstraint.sourceFile
      line := diag.targetConstraint.sourceLine
      name := hopTarget
    }
    path := pathNames
    hop := [hopSource, hopTarget]
    site := { file := diag.siteFile, line := diag.siteLine }
    guaranteeFile := diag.sourceConstraint.sourceFile
    guaranteeLine := diag.sourceConstraint.sourceLine
    guaranteeAtFile := diag.sourceConstraint.originOrOwn.1
    guaranteeAtLine := diag.sourceConstraint.originOrOwn.2
    sourceGuarantee := formatBound diag.sourceConstraint
    targetRequirement := formatBound diag.targetConstraint
    verificationLevel := formatVerificationLevel verLevel
    suggestion := diag.suggestion
  }

/-- Generate a warning for an unresolved dependent expression on a hop's
    source node (`nodeName`); `hopTarget` is the node it flows into and `pre`
    the precondition of `hopTarget` (same kind) that the unresolved bound
    would be checked against. -/
def unresolvedDepWarning (constraint : Constraint) (nodeName : String)
    (hopTarget : String := "") (pre : Constraint := constraint) : DiagnosticInfo :=
  { severity := .warning
    sourceConstraint := constraint
    targetConstraint := pre
    path := [nodeName]
    suggestion := s!"Dependent expression on {nodeName} could not be resolved. " ++
                  s!"Check that upstream postconditions provide the required input bindings."
    hopSource := nodeName
    hopTarget := hopTarget }

/-- Generate a warning for a precondition of `hopTarget` whose kind the hop
    source (`sourceNodeName`, after composition) has no postcondition for. -/
def missingPostconditionWarning (pre : Constraint) (sourceNodeName : String)
    (hopTarget : String := "") : DiagnosticInfo :=
  { severity := .warning
    -- No source constraint exists; report the kind with no bound
    -- ("precision (unspecified)") at the requirement's location.
    sourceConstraint := { pre with staticBound := none, depExpr := none,
                                   typeName := none, choicesList := none }
    targetConstraint := pre
    path := [sourceNodeName]
    suggestion := s!"'{sourceNodeName}' has no " ++
                  s!"{formatConstraintKind pre.kind} postcondition for the value it passes " ++
                  s!"to '{hopTarget}'. Its {formatConstraintKind pre.kind} requirements pass " ++
                  s!"vacuously — this may hide real inconsistencies."
    hopSource := sourceNodeName
    hopTarget := hopTarget }

end ContractGraph
