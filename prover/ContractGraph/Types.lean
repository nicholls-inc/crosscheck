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

/-- Decimal display of a bound held as value × 10^scale:
    `formatScaled 500000 6 = "0.5"`, `formatScaled (-1000000) 6 = "-1"`. -/
def formatScaled (m : Int) (scale : Nat) : String :=
  let sign := if m < 0 then "-" else ""
  let a := m.natAbs
  let ip := a / 10 ^ scale
  let fp := a % 10 ^ scale
  if fp == 0 then s!"{sign}{ip}"
  else
    let raw := toString fp
    let digits := "".pushn '0' (scale - raw.length) ++ raw  -- `scale` digits, zero-padded
    let trimmed := (digits.toList.reverse.dropWhile (· == '0')).reverse
    s!"{sign}{ip}.{String.ofList trimmed}"

/-- Decimal display of a bound held in micros (value × 10^6). -/
def formatMicros (m : Int) : String := formatScaled m 6

/-- Display a static bound of kind `k`: range bounds are held as
    value × 10^scale (`Constraint.scale`, 6 = micros by default). -/
def formatBoundValue (k : ConstraintKind) (b : Int) (scale : Nat := 6) : String :=
  match k with
  | .range | .rangeMin => formatScaled b scale
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
  /-- For `range` / `rangeMin`: the bound (and dependent-expression
      literals) are value × 10^scale. Every range constraint of a graph read
      from a database has the same scale (`rangeScale`). -/
  scale             : Nat := 6
  /-- Where the bound comes from, for display (`"guarantee_at"`): for a bound
      produced by composing a dependent expression, the input constraint
      whose value the result equals. Empty: the constraint's own location.
      Checking never reads it (`checkConstraintPair_withOrigin`). -/
  originFile        : String := ""
  originLine        : Nat := 0
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
  /-- A call-site node (`nodes.is_call_site`). A path whose head is a call
      site with an incoming checked edge is a suffix of longer paths: its
      warnings are not reported. -/
  isCallSite      : Bool := false
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
  /-- Where the violating guarantee comes from (the origin of a composed
      bound, else the source constraint's location). Printed as
      `"guarantee_at"` for errors; not used by deduplication. -/
  guaranteeAtFile : String := ""
  guaranteeAtLine : Nat := 0
  sourceGuarantee : String
  targetRequirement : String
  verificationLevel : String
  suggestion : String
  deriving Repr

/-- JSON-compatible summary. -/
structure Summary where
  contractsChecked : Nat
  edgesChecked : Nat
  /-- Path-based checker: data paths checked. State-based checker: hop
      states checked (the same as `statesChecked`, kept for compatibility). -/
  pathsChecked : Nat
  /-- Hop states checked (state-based checker; 0 for the path-based one). -/
  statesChecked : Nat := 0
  deriving Repr

/-- Full output structure. -/
structure CheckOutput where
  summary : Summary
  results : List ResultEntry
  exitCode : Nat
  deriving Repr

end ContractGraph
