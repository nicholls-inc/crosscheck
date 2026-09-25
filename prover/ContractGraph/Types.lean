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
  | rangeMin  -- lower bound on a numeric value (`range` is the upper bound)
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
    | .rangeMin    => "range_min"

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

/-- Decimal display of a bound held in micros (value × 10^6), as used for
    `range` / `rangeMin` bounds: `500000 ↦ "0.5"`, `-1000000 ↦ "-1"`. -/
def formatMicros (m : Int) : String :=
  let sign := if m < 0 then "-" else ""
  let a := m.natAbs
  let ip := a / 1000000
  let fp := a % 1000000
  if fp == 0 then s!"{sign}{ip}"
  else
    let raw := toString fp
    let digits := "".pushn '0' (6 - raw.length) ++ raw  -- six digits, zero-padded
    let trimmed := (digits.toList.reverse.dropWhile (· == '0')).reverse
    s!"{sign}{ip}.{String.ofList trimmed}"

/-- Display a static bound of kind `k`: range bounds are in micros. -/
def formatBoundValue (k : ConstraintKind) (b : Int) : String :=
  match k with
  | .range | .rangeMin => formatMicros b
  | _ => toString b

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
  /-- For a function precondition: the parameter it constrains. `none` means
      every parameter (legacy rows). Postconditions leave it `none`. -/
  subject           : Option String := none
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
  /-- Location of the node's definition (`nodes.source_file/source_line`);
      empty/0 when unknown. -/
  sourceFile      : String := ""
  sourceLine      : Nat := 0
  deriving Repr

/-- Relationship types for edges. -/
inductive Relationship where
  | calls
  | writesTo
  | flowsTo  -- the source function's result is passed as an argument to the target
  deriving Repr, BEq

/-- An edge in the contract graph. -/
structure Edge where
  source : Node
  target : Node
  relationship : Relationship
  /-- Location of the write or call expression that produced the edge
      (`edges.site_file/site_line`); empty/0 when unknown. -/
  siteFile : String := ""
  siteLine : Nat := 0
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
  /-- Name of the source node of the hop that produced this diagnostic
      (after composition: the intermediate node). Empty when not tagged. -/
  hopSource        : String := ""
  /-- Name of the node whose precondition failed (the hop target).
      Empty when not tagged. -/
  hopTarget        : String := ""
  /-- Location of the hop source node (after composition: the intermediate
      node's definition). Empty/0 when not tagged. -/
  hopSourceFile    : String := ""
  hopSourceLine    : Nat := 0
  /-- Site (write or call expression) of the hop's edge. Empty/0 when unknown. -/
  siteFile         : String := ""
  siteLine         : Nat := 0
  deriving Repr

/-- Result of a consistency check. -/
inductive CheckResult where
  | consistent : CheckResult
  | inconsistent : DiagnosticInfo → CheckResult
  deriving Repr

/-- An inconsistency of severity error. Warnings and consistent results are
    not errors; only errors affect the exit code. -/
def CheckResult.isError : CheckResult → Bool
  | .consistent => false
  | .inconsistent d =>
    match d.severity with
    | .error => true
    | .warning => false

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

/-- Location of the write or call expression of a hop (`"site"` in JSON). -/
structure SiteLocation where
  file : String := ""
  line : Nat := 0
  deriving Repr

/-- JSON-compatible result entry. -/
structure ResultEntry where
  status : String
  severity : String
  /-- Errors: the path head node (name and definition). Warnings: the hop
      source node. -/
  source : SourceLocation
  target : SourceLocation
  path : List String
  /-- `[hop source name, hop target name]` of the failing hop. -/
  hop : List String := []
  /-- Site of the failing hop's edge. -/
  site : SiteLocation := {}
  /-- Location of the source constraint (the guarantee). Not printed; used
      by deduplication, which must not depend on the path head. -/
  guaranteeFile : String := ""
  guaranteeLine : Nat := 0
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
