-- Types.lean

namespace ContractGraph

/-- Constraint kinds corresponding to the SQLite constraint_type column. -/
inductive ConstraintKind where
  | precision
  | nullability
  | type
  | range
  | length
  | choices
  deriving Repr, BEq, Hashable, Inhabited, DecidableEq

/-- Explicit ToString ensures s!"input_{kind}" produces "input_precision", etc.,
    matching the dependent_expr grammar convention. -/
instance : ToString ConstraintKind where
  toString
    | .precision   => "precision"
    | .nullability => "nullability"
    | .type        => "type"
    | .range       => "range"
    | .length      => "length"
    | .choices     => "choices"

/-- Verification levels, ordered from strongest to weakest. -/
inductive VerificationLevel where
  | proved
  | tested
  | extracted
  | assumed
  deriving Repr, BEq, Hashable, Inhabited

/-- Ordering on verification levels for weakest-link computation. -/
instance : Ord VerificationLevel where
  compare a b := match a, b with
    | .proved, .proved => .eq
    | .proved, _ => .gt
    | _, .proved => .lt
    | .tested, .tested => .eq
    | .tested, _ => .gt
    | _, .tested => .lt
    | .extracted, .extracted => .eq
    | .extracted, _ => .gt
    | _, .extracted => .lt
    | .assumed, .assumed => .eq

instance : Min VerificationLevel where
  min a b := if Ord.compare a b == .lt then a else b

instance : Max VerificationLevel where
  max a b := if Ord.compare a b == .gt then a else b

/-- A dependent expression: postconditions that are functions of inputs. -/
inductive DepExpr where
  | lit    : Int → DepExpr
  | input  : String → DepExpr
  | max    : DepExpr → DepExpr → DepExpr
  | min    : DepExpr → DepExpr → DepExpr
  | add    : DepExpr → DepExpr → DepExpr
  | sub    : DepExpr → DepExpr → DepExpr
  deriving Repr, BEq, Inhabited

/-- A contract constraint. -/
structure Constraint where
  kind              : ConstraintKind
  staticBound       : Option Int := none
  depExpr           : Option DepExpr := none
  typeName          : Option String := none
  choicesList       : Option (List String) := none
  sourceFile        : String
  sourceLine        : Nat
  verificationLevel : VerificationLevel
  deriving Repr

/-- Contract role: precondition or postcondition. -/
inductive ContractRole where
  | precondition
  | postcondition
  deriving Repr, BEq, Inhabited

/-- A node in the contract graph. -/
structure Node where
  id              : Nat
  name            : String
  kind            : String
  preconditions   : List Constraint
  postconditions  : List Constraint
  deriving Repr

/-- Relationship types for edges. -/
inductive Relationship where
  | calls
  | writesTo
  deriving Repr, BEq

/-- An edge in the contract graph. -/
structure Edge where
  source : Node
  target : Node
  relationship : Relationship
  deriving Repr

/-- Severity levels for diagnostics. -/
inductive Severity where
  | error
  | warning
  deriving Repr, BEq, Inhabited

instance : ToString Severity where
  toString
    | .error => "error"
    | .warning => "warning"

/-- Diagnostic information for an inconsistency. -/
structure DiagnosticInfo where
  severity         : Severity
  sourceConstraint : Constraint
  targetConstraint : Constraint
  path             : List String
  suggestion       : String
  deriving Repr

/-- Result of a consistency check. -/
inductive CheckResult where
  | consistent : CheckResult
  | inconsistent : DiagnosticInfo → CheckResult
  deriving Repr

/-- The full contract graph. -/
structure ContractGraph where
  nodes : List Node
  edges : List Edge
  deriving Repr

/-- Source location for diagnostics. -/
structure SourceLocation where
  file : String
  line : Nat
  name : String
  deriving Repr

/-- JSON-compatible result entry. -/
structure ResultEntry where
  status : String
  severity : String
  source : SourceLocation
  target : SourceLocation
  path : List String
  sourceGuarantee : String
  targetRequirement : String
  verificationLevel : String
  suggestion : String
  deriving Repr

/-- JSON-compatible summary. -/
structure Summary where
  contractsChecked : Nat
  edgesChecked : Nat
  pathsChecked : Nat
  deriving Repr

/-- Full output structure. -/
structure CheckOutput where
  summary : Summary
  results : List ResultEntry
  exitCode : Nat
  deriving Repr

end ContractGraph
