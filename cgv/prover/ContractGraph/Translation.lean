-- Translation.lean
--
-- SQLite → Lean proposition translation.
-- Uses leanprover/leansqlite low-level API to read the contract graph
-- from the SQLite database produced by Layer 1.

import ContractGraph.Types
import ContractGraph.DependentExpr
import SQLite
import Std.Data.HashMap

namespace ContractGraph

open SQLite

/-- Parse a constraint kind string from the database; `none` if unknown.
    `range_min` is not a schema value (`range` rows carry lower bounds) but
    is kept for rows written by hand. -/
def parseConstraintKind : String → Option ConstraintKind
  | "precision"   => some .precision
  | "nullability" => some .nullability
  | "type"        => some .type
  | "range"       => some .range
  | "length"      => some .length
  | "choices"     => some .choices
  | "range_min"   => some .rangeMin
  | _             => none

/-- Parse a verification level string from the database; `none` if unknown. -/
def parseVerifLevel : String → Option VerificationLevel
  | "PROVED"    => some .proved
  | "TESTED"    => some .tested
  | "EXTRACTED" => some .extracted
  | "ASSUMED"   => some .assumed
  | _           => none

/-- Parse a contract role string from the database; `none` if unknown. -/
def parseContractRole : String → Option ContractRole
  | "precondition"  => some .precondition
  | "postcondition" => some .postcondition
  | _               => none

/-- Parse a relationship string from the database; `none` if unknown. -/
def parseRelationship : String → Option Relationship
  | "calls"     => some .calls
  | "writes_to" => some .writesTo
  | "flows_to"  => some .flowsTo
  | _           => none

/-- The node kinds of the schema. The checker starts paths only at
    `function` nodes and ends them only at `model` nodes, so a misspelt kind
    would drop the node from every path it starts or ends. -/
def nodeKinds : List String := ["model", "function", "field"]

/-- Helper to read an optional integer column (returns none if NULL). -/
private def readOptionalInt (stmt : Stmt) (col : Int32) : IO (Option Int) := do
  let isNull ← stmt.columnNull col
  if isNull then
    return none
  else
    let v ← stmt.columnInt64 col
    return some v.toInt

/-- Helper to read an optional string column (returns none if NULL or empty). -/
private def readOptionalString (stmt : Stmt) (col : Int32) : IO (Option String) := do
  let isNull ← stmt.columnNull col
  if isNull then
    return none
  else
    let s ← stmt.columnText col
    if s.isEmpty then return none else return some s

/-- Helper to read a nullable string column, keeping an empty string. -/
private def readNullableText (stmt : Stmt) (col : Int32) : IO (Option String) := do
  if ← stmt.columnNull col then return none else return some (← stmt.columnText col)

/-- One `contracts` row, with the columns the checker reads. -/
structure ContractRow where
  nodeId            : Nat
  constraintType    : String
  decimalPlaces     : Option Int := none
  maxLength         : Option Int := none
  nullable          : Option Int := none
  typeName          : Option String := none
  /-- Integer bounds in plain units (Lean-side tests only; `readContracts`
      fills `minMicros`/`maxMicros`). Used when the micros field is `none`. -/
  minValue          : Option Int := none
  maxValue          : Option Int := none
  /-- Bounds × 10^6 (`param_min_micros`/`param_max_micros`). -/
  minMicros         : Option Int := none
  maxMicros         : Option Int := none
  /-- Exact decimal bounds (`param_min_decimal`/`param_max_decimal`,
      `-?digits(.digits)?`). Preferred over micros, then REAL. -/
  minDecimal        : Option String := none
  maxDecimal        : Option String := none
  /-- Legacy REAL bounds (`param_min_value`/`param_max_value`), used when a
      bound has neither a decimal nor a micros value. -/
  minReal           : Option Float := none
  maxReal           : Option Float := none
  choices           : Option String := none
  sourceFile        : String := ""
  sourceLine        : Nat := 0
  verificationLevel : String := "EXTRACTED"
  role              : Option String := none
  dependentExpr     : Option String := none
  /-- Parameter a precondition constrains; `none` = every parameter. -/
  subject           : Option String := none
  /-- `some e`: a per-edge postcondition of edge `e`'s source, not a node contract. -/
  edgeId            : Option Nat := none
  deriving Repr

/-- One `nodes` row. -/
structure NodeRow where
  id         : Nat
  name       : String
  kind       : String
  sourceFile : String := ""
  sourceLine : Nat := 0
  /-- `nodes.is_call_site` (0 when the column is absent). -/
  isCallSite : Bool := false
  deriving Repr

/-- `(id, name, kind)` without a location (Lean-side tests). -/
instance : Coe (Nat × String × String) NodeRow where
  coe | (id, name, kind) => { id := id, name := name, kind := kind }

/-- One `edges` row. -/
structure EdgeRow where
  id             : Nat
  sourceId       : Nat
  targetId       : Nat
  /-- `relationship` as stored; `translateRows` parses it. -/
  relationship   : String
  /-- `some p`: only the target's preconditions with subject `p` (or none) apply. -/
  targetParam    : Option String := none
  /-- The source's postconditions are replaced by the rows with this edge's id. -/
  sourceOverride : Bool := false
  /-- Location of the write or call expression (`site_file`/`site_line`). -/
  siteFile       : String := ""
  siteLine       : Nat := 0
  deriving Repr

/-! ## `param_choices`: a JSON array of strings, or a legacy comma list -/

private def hexVal (c : Char) : Option Nat :=
  if '0' ≤ c && c ≤ '9' then some (c.toNat - '0'.toNat)
  else if 'a' ≤ c && c ≤ 'f' then some (c.toNat - 'a'.toNat + 10)
  else if 'A' ≤ c && c ≤ 'F' then some (c.toNat - 'A'.toNat + 10)
  else none

/-- Parse the body of a JSON string (after the opening quote) up to the
    closing quote; returns the string and the rest. -/
private def parseJsonStringBody : List Char → String → Option (String × List Char)
  | [], _ => none
  | '"' :: rest, acc => some (acc, rest)
  | '\\' :: c :: rest, acc =>
    match c with
    | '"' => parseJsonStringBody rest (acc.push '"')
    | '\\' => parseJsonStringBody rest (acc.push '\\')
    | '/' => parseJsonStringBody rest (acc.push '/')
    | 'n' => parseJsonStringBody rest (acc.push '\n')
    | 't' => parseJsonStringBody rest (acc.push '\t')
    | 'r' => parseJsonStringBody rest (acc.push '\r')
    | 'b' => parseJsonStringBody rest (acc.push (Char.ofNat 8))
    | 'f' => parseJsonStringBody rest (acc.push (Char.ofNat 12))
    | 'u' =>
      match rest with
      | a :: b :: c :: d :: rest' =>
        match hexVal a, hexVal b, hexVal c, hexVal d with
        | some a, some b, some c, some d =>
          parseJsonStringBody rest' (acc.push (Char.ofNat (((a * 16 + b) * 16 + c) * 16 + d)))
        | _, _, _, _ => none
      | _ => none
    | _ => none
  | c :: rest, acc => parseJsonStringBody rest (acc.push c)

private def skipWs : List Char → List Char
  | c :: rest => if c == ' ' || c == '\n' || c == '\t' || c == '\r' then skipWs rest else c :: rest
  | [] => []

/-- Parse `"s1", "s2", ... ]` (after `[`). -/
private def parseJsonItems : (fuel : Nat) → List Char → List String → Option (List String)
  | 0, _, _ => none
  | fuel + 1, cs, acc =>
    match skipWs cs with
    | ']' :: rest => if (skipWs rest).isEmpty && acc.isEmpty then some [] else none
    | '"' :: rest =>
      match parseJsonStringBody rest "" with
      | none => none
      | some (s, rest) =>
        match skipWs rest with
        | ',' :: rest => parseJsonItems fuel rest (acc ++ [s])
        | ']' :: rest => if (skipWs rest).isEmpty then some (acc ++ [s]) else none
        | _ => none
    | _ => none

/-- A JSON array of strings (`["a", "b"]`); `none` if malformed. -/
def parseJsonStringArray (s : String) : Option (List String) :=
  match skipWs s.toList with
  | '[' :: rest => parseJsonItems (rest.length + 1) rest []
  | _ => none

/-- `param_choices`: a JSON array of strings when it starts with `[`
    (`none` if malformed; `translateRows` rejects such a row, see
    `ContractRow.malformed`), otherwise the legacy
    comma-separated list. -/
def parseChoices (s : String) : Option (List String) :=
  if s.trimAscii.toString.startsWith "[" then parseJsonStringArray s
  else some (s.splitOn ",")

/-! ## Range bounds, scaled exactly

Range bounds are held as integers value × 10^D, where D (`rangeScale`) is
the largest number of decimal places among the exact decimal bounds of the
database, and at least 6 when some bound is only given in micros or as a
REAL. Every range constraint of a graph has the same scale, so comparisons
are exact for decimal and micros bounds of any size. -/

/-- Scale the literals of a dependent expression by `k`. Range bounds are
    held as value × 10^D while `dependent_expr` literals are in plain
    units; `max`/`min`/`add`/`sub` commute with scaling, so scaling the
    literals gives the expression at scale D. -/
def DepExpr.scaleLits (k : Int) : DepExpr → DepExpr
  | .lit n => .lit (n * k)
  | .input s => .input s
  | .max a b => .max (a.scaleLits k) (b.scaleLits k)
  | .min a b => .min (a.scaleLits k) (b.scaleLits k)
  | .add a b => .add (a.scaleLits k) (b.scaleLits k)
  | .sub a b => .sub (a.scaleLits k) (b.scaleLits k)

/-- The value of a non-empty list of decimal digits. -/
def digitsValue (cs : List Char) : Option Nat :=
  if !cs.isEmpty && cs.all Char.isDigit then
    some (cs.foldl (fun n c => 10 * n + (c.toNat - '0'.toNat)) 0)
  else none

/-- Parse an exact decimal `-?digits(.digits)?` into `(m, f)` with value
    `m / 10^f`; `none` if malformed (then the micros/REAL columns are used). -/
def parseDecimal (s : String) : Option (Int × Nat) :=
  let cs := s.toList
  let (neg, body) := match cs with
    | '-' :: rest => (true, rest)
    | _ => (false, cs)
  let sign (n : Nat) : Int := if neg then -(n : Int) else n
  let ip := body.takeWhile (· != '.')
  match body.dropWhile (· != '.') with
  | [] => (digitsValue ip).map fun n => (sign n, 0)
  | _ :: fp =>
    match digitsValue ip, digitsValue fp with
    | some i, some f => some (sign (i * 10 ^ fp.length + f), fp.length)
    | _, _ => none

/-- `m / 10^src` at scale `dst`: exact when `dst ≥ src`; otherwise rounded
    up (`up`) or down. -/
def rescale (m : Int) (src dst : Nat) (up : Bool) : Int :=
  if src ≤ dst then m * 10 ^ (dst - src)
  else
    let d : Int := 10 ^ (src - dst)
    if up then -((-m) / d) else m / d

/-- Convert a legacy REAL bound to value × 10^scale, rounding conservatively:
    a requirement (`strict = true`) to the stricter side, a guarantee to the
    weaker side. `upper` says whether the bound is an upper bound. A value
    within 10^-4 of an integer after scaling (floating-point noise, e.g.
    `0.3 × 10^6`) is taken as that integer. -/
def legacyScaled (v : Float) (upper strict : Bool) (scale : Nat) : Int :=
  let x := v * (10 ^ scale : Nat).toFloat
  let r := x.round
  if (x - r).abs < 0.0001 then r.toInt64.toInt
  else
    -- stricter: upper ↓, lower ↑; weaker: upper ↑, lower ↓
    if upper == strict then x.floor.toInt64.toInt else x.ceil.toInt64.toInt

/-- `legacyScaled` at scale 6 (micros). -/
def legacyMicros (v : Float) (upper strict : Bool) : Int := legacyScaled v upper strict 6

/-- Whether a row states a guarantee (a postcondition) rather than a
    requirement (a precondition, including `contract_role` NULL). -/
def ContractRow.isGuarantee (row : ContractRow) : Bool := row.role == some "postcondition"

/-- A bound at scale `scale`: the exact decimal, else micros, else the REAL,
    else the plain-unit integer (Lean-side tests). A bound that cannot be
    represented exactly is rounded to the requirement's stricter side and
    the guarantee's weaker side. -/
def scaledBound (dec : Option String) (micros : Option Int) (real : Option Float)
    (plain : Option Int) (upper strict : Bool) (scale : Nat) : Option Int :=
  let up := upper != strict
  match dec.bind parseDecimal with
  | some (m, f) => some (rescale m f scale up)
  | none =>
    match micros with
    | some m => some (rescale m 6 scale up)
    | none =>
      match real with
      | some v => some (legacyScaled v upper strict scale)
      | none => plain.map (· * 10 ^ scale)

/-- A row's lower bound at scale `scale`. -/
def ContractRow.lowerScaled (row : ContractRow) (scale : Nat) : Option Int :=
  scaledBound row.minDecimal row.minMicros row.minReal row.minValue false (!row.isGuarantee) scale

/-- A row's upper bound at scale `scale`. -/
def ContractRow.upperScaled (row : ContractRow) (scale : Nat) : Option Int :=
  scaledBound row.maxDecimal row.maxMicros row.maxReal row.maxValue true (!row.isGuarantee) scale

/-- Where a row came from, for error messages. -/
def ContractRow.loc (row : ContractRow) : String :=
  s!"{row.sourceFile}:{row.sourceLine} ({row.constraintType})"

/-- Whether a row has the value that `translateContractRow` reads for `kind`. -/
def ContractRow.hasValue (row : ContractRow) : ConstraintKind → Bool
  | .precision => row.decimalPlaces.isSome
  | .length => row.maxLength.isSome
  | .nullability => row.nullable.isSome
  | .type => row.typeName.isSome
  | .choices => row.choices.isSome
  | .rangeMin => row.minDecimal.isSome || row.minMicros.isSome || row.minReal.isSome ||
      row.minValue.isSome
  | .range => row.minDecimal.isSome || row.minMicros.isSome || row.minReal.isSome ||
      row.minValue.isSome || row.maxDecimal.isSome || row.maxMicros.isSome ||
      row.maxReal.isSome || row.maxValue.isSome

/-- Why a contract row's values cannot be translated faithfully, if they
    cannot: a `param_choices` value that starts like a JSON array but is not
    one, an exact decimal bound that is not a decimal, a `dependent_expr`
    that does not parse, or a row with neither the value its kind reads nor
    a `dependent_expr`. Translation would read each as no constraint at all
    (dropping it silently), so `translateRows` rejects the rows instead
    (exit code 2). -/
def ContractRow.malformed (row : ContractRow) : Option String :=
  let loc := row.loc
  if row.choices.any (fun s => (parseChoices s).isNone) then
    some s!"malformed param_choices at {loc}"
  else if row.minDecimal.any (fun s => (parseDecimal s).isNone) then
    some s!"malformed param_min_decimal at {loc}"
  else if row.maxDecimal.any (fun s => (parseDecimal s).isNone) then
    some s!"malformed param_max_decimal at {loc}"
  else if row.dependentExpr.any (fun e => (parseDepExpr e).isNone) then
    some s!"malformed dependent_expr at {loc}"
  else if row.dependentExpr.isNone &&
      (parseConstraintKind row.constraintType).any (fun k => !row.hasValue k) then
    some s!"no value and no dependent_expr at {loc}"
  else none

/-- A row's lower bound in micros. -/
def ContractRow.lowerMicros (row : ContractRow) : Option Int := row.lowerScaled 6

/-- A row's upper bound in micros. -/
def ContractRow.upperMicros (row : ContractRow) : Option Int := row.upperScaled 6

/-- The decimal places one bound needs: its exact decimal's, else 6 for a
    micros or REAL bound, else 0. -/
def boundPlaces (dec : Option String) (micros : Option Int) (real : Option Float) : Nat :=
  match dec.bind parseDecimal with
  | some (_, f) => f
  | none => if micros.isSome || real.isSome then 6 else 0

/-- The scale D of a database's range bounds: the most decimal places any
    range bound needs (`boundPlaces`). -/
def rangeScale (rows : List ContractRow) : Nat :=
  rows.foldl (fun d row =>
    match parseConstraintKind row.constraintType with
    | some .range | some .rangeMin =>
      max d (max (boundPlaces row.minDecimal row.minMicros row.minReal)
                 (boundPlaces row.maxDecimal row.maxMicros row.maxReal))
    | _ => d) 0

/-- Translate a contracts row into Lean constraints (pure).

    Translation rules (SQL row → Lean Constraint):
    - precision:   staticBound ← param_decimal_places
    - nullability: staticBound ← param_nullable (0=NOT NULL, 1=NULL)
    - type:        typeName ← param_type_name
    - range:       up to two constraints: `range` (upper) with
                   staticBound ← the upper bound, and `rangeMin` (lower) with
                   staticBound ← the lower bound, both × 10^scale
                   (`lowerScaled`/`upperScaled`: exact decimal, else micros,
                   else REAL). A row with neither yields one `range`
                   constraint without a bound. The literals of a range
                   `dependent_expr` are scaled by 10^scale
                   (`DepExpr.scaleLits`), and every constraint records
                   `scale`.
    - length:      staticBound ← param_max_length
    - choices:     choicesList ← `parseChoices param_choices` (JSON array of
                   strings, or legacy comma list)
    `dependent_expr` goes on the upper `range` constraint, or on `rangeMin`
    when the row has only a lower bound. `subject` is copied to every
    constraint. An unknown `constraint_type` or `verification_level`, or a
    `malformed` row, is an error. -/
def translateContractRow (row : ContractRow) (scale : Nat := 6) : Except String (List Constraint) := do
  let some kind := parseConstraintKind row.constraintType
    | throw s!"unknown constraint_type at {row.loc}"
  let some level := parseVerifLevel row.verificationLevel
    | throw s!"unknown verification_level '{row.verificationLevel}' at {row.loc}"
  if let some msg := row.malformed then throw msg
  let depExpr := row.dependentExpr.bind parseDepExpr
  let depExpr := match kind with
    | .range | .rangeMin => depExpr.map (·.scaleLits (10 ^ scale))
    | _ => depExpr
  let base : Constraint := {
    kind := kind
    depExpr := depExpr
    sourceFile := row.sourceFile
    sourceLine := row.sourceLine
    verificationLevel := level
    subject := row.subject
    scale := scale
  }
  return match kind with
  | .precision => [{ base with staticBound := row.decimalPlaces }]
  | .nullability => [{ base with staticBound := row.nullable }]
  | .length => [{ base with staticBound := row.maxLength }]
  | .type => [{ base with typeName := row.typeName }]
  | .choices => [{ base with choicesList := row.choices.bind parseChoices }]
  | .rangeMin => [{ base with staticBound := row.lowerScaled scale }]
  | .range =>
    match row.lowerScaled scale, row.upperScaled scale with
    | some lo, some hi =>
      [{ base with staticBound := some hi },
       { base with kind := .rangeMin, staticBound := some lo, depExpr := none }]
    | some lo, none => [{ base with kind := .rangeMin, staticBound := some lo }]
    | none, hi => [{ base with staticBound := hi }]

/-- The role of a contracts row (`contract_role` NULL counts as precondition);
    an unknown role is an error. -/
def ContractRow.contractRole (row : ContractRow) : Except String ContractRole :=
  match row.role with
  | none => .ok .precondition
  | some r =>
    match parseContractRole r with
    | some role => .ok role
    | none => .error s!"unknown contract_role '{r}' at {row.loc}"

/-- Whether `table` has a column named `column` (so older databases without
    the data-flow v2 columns still translate). -/
private def hasColumn (db : SQLite) (table column : String) : IO Bool := do
  let stmt ← prepare db s!"PRAGMA table_info({table})"
  let mut found := false
  let mut hasRow ← stmt.step
  while hasRow do
    let name ← stmt.columnText 1
    if name == column then found := true
    hasRow ← stmt.step
  return found

/-- `column` if the table has it, else `NULL` (for older databases). -/
private def columnOrNull (db : SQLite) (table column : String) : IO String := do
  if ← hasColumn db table column then return column else return s!"NULL AS {column}"

/-- Read an optional REAL column. -/
private def readOptionalFloat (stmt : Stmt) (col : Int32) : IO (Option Float) := do
  if ← stmt.columnNull col then return none
  else return some (← stmt.columnDouble col)

/-- Read all contracts rows.

    Selected column layout (0-indexed):
      0: id, 1: node_id, 2: constraint_type,
      3: param_max_digits, 4: param_decimal_places,
      5: param_max_length, 6: param_nullable,
      7: param_type_name, 8: param_min_value,
      9: param_max_value, 10: param_choices,
      11: source_file, 12: source_line,
      13: is_implicit, 14: verification_level,
      15: contract_role, 16: dependent_expr,
      17: subject, 18: edge_id,
      19: param_min_micros, 20: param_max_micros,
      21: param_min_decimal, 22: param_max_decimal

    Range bounds are kept as read (decimal strings, micros, REALs); they are
    scaled in `translateContractRow` (see `rangeScale`). Columns missing in
    older databases read as NULL. -/
def readContracts (db : SQLite) : IO (List ContractRow) := do
  let subjectCol ← columnOrNull db "contracts" "subject"
  let edgeIdCol ← columnOrNull db "contracts" "edge_id"
  let minMicrosCol ← columnOrNull db "contracts" "param_min_micros"
  let maxMicrosCol ← columnOrNull db "contracts" "param_max_micros"
  let minDecimalCol ← columnOrNull db "contracts" "param_min_decimal"
  let maxDecimalCol ← columnOrNull db "contracts" "param_max_decimal"
  let stmt ← prepare db
    ("SELECT id, node_id, constraint_type, param_max_digits, param_decimal_places, " ++
     "param_max_length, param_nullable, param_type_name, param_min_value, param_max_value, " ++
     "param_choices, source_file, source_line, is_implicit, verification_level, " ++
     s!"contract_role, dependent_expr, {subjectCol}, {edgeIdCol}, {minMicrosCol}, " ++
     s!"{maxMicrosCol}, {minDecimalCol}, {maxDecimalCol} FROM contracts ORDER BY id")
  let mut results : Array ContractRow := #[]
  let mut hasRow ← stmt.step
  while hasRow do
    let nodeId ← stmt.columnInt64 1
    let role ← readNullableText stmt 15
    let legacyMin ← readOptionalFloat stmt 8
    let legacyMax ← readOptionalFloat stmt 9
    let minMicros ← readOptionalInt stmt 19
    let maxMicros ← readOptionalInt stmt 20
    let row : ContractRow := {
      nodeId := nodeId.toInt.toNat
      constraintType := ← stmt.columnText 2
      decimalPlaces := ← readOptionalInt stmt 4
      maxLength := ← readOptionalInt stmt 5
      nullable := ← readOptionalInt stmt 6
      typeName := ← readOptionalString stmt 7
      minMicros := minMicros
      maxMicros := maxMicros
      minDecimal := ← readOptionalString stmt 21
      maxDecimal := ← readOptionalString stmt 22
      minReal := legacyMin
      maxReal := legacyMax
      choices := ← readOptionalString stmt 10
      sourceFile := ← stmt.columnText 11
      sourceLine := (← stmt.columnInt64 12).toInt.toNat
      verificationLevel := ← stmt.columnText 14
      role := role
      dependentExpr := ← readOptionalString stmt 16
      subject := ← readOptionalString stmt 17
      edgeId := (← readOptionalInt stmt 18).map Int.toNat
    }
    results := results.push row
    hasRow ← stmt.step
  return results.toList

/-- Read all nodes from the database, with their definition locations.
    `qualified_name` is not read. -/
def readNodes (db : SQLite) : IO (List NodeRow) := do
  let callSiteCol ← columnOrNull db "nodes" "is_call_site"
  let stmt ← prepare db
    s!"SELECT id, name, kind, source_file, source_line, {callSiteCol} FROM nodes ORDER BY id"
  let mut results : Array NodeRow := #[]
  let mut hasRow ← stmt.step
  while hasRow do
    let nodeId ← stmt.columnInt64 0
    let name ← stmt.columnText 1
    let kind ← stmt.columnText 2
    let file := (← readOptionalString stmt 3).getD ""
    let line := ((← readOptionalInt stmt 4).getD 0).toNat
    let callSite := ((← readOptionalInt stmt 5).getD 0) != 0
    results := results.push
      { id := nodeId.toInt.toNat, name := name, kind := kind, sourceFile := file, sourceLine := line,
        isCallSite := callSite }
    hasRow ← stmt.step
  return results.toList

/-- Read all edges from the database. -/
def readEdges (db : SQLite) : IO (List EdgeRow) := do
  let targetParamCol ← columnOrNull db "edges" "target_param"
  let overrideCol ← columnOrNull db "edges" "source_override"
  let siteFileCol ← columnOrNull db "edges" "site_file"
  let siteLineCol ← columnOrNull db "edges" "site_line"
  let stmt ← prepare db
    (s!"SELECT id, source_node_id, target_node_id, relationship, {targetParamCol}, " ++
     s!"{overrideCol}, {siteFileCol}, {siteLineCol} FROM edges ORDER BY id")
  let mut results : Array EdgeRow := #[]
  let mut hasRow ← stmt.step
  while hasRow do
    let edgeId ← stmt.columnInt64 0
    let srcId ← stmt.columnInt64 1
    let tgtId ← stmt.columnInt64 2
    let rel ← stmt.columnText 3
    let targetParam ← readOptionalString stmt 4
    let override ← readOptionalInt stmt 5
    let siteFile ← readOptionalString stmt 6
    let siteLine ← readOptionalInt stmt 7
    results := results.push {
      id := edgeId.toInt.toNat
      sourceId := srcId.toInt.toNat
      targetId := tgtId.toInt.toNat
      relationship := rel
      targetParam := targetParam
      sourceOverride := override.getD 0 != 0
      siteFile := siteFile.getD ""
      siteLine := (siteLine.getD 0).toNat }
    hasRow ← stmt.step
  return results.toList

/-- Whether a precondition applies to the argument bound to `param`:
    its subject is that parameter, or it has no subject (all parameters). -/
def appliesToParam (param : String) (c : Constraint) : Bool :=
  match c.subject with
  | none => true
  | some s => s == param

/-- Group `xs` by `key`, keeping the order of `xs` within each group. -/
def groupBy {α : Type} (key : α → Option Nat) (xs : List α) : Std.HashMap Nat (List α) :=
  xs.foldr (fun x m =>
    match key x with
    | some k => m.insert k (x :: m.getD k [])
    | none => m) ∅

/-- The first id that occurs twice in `ids`, if any. -/
def firstDuplicate (ids : List Nat) : Option Nat :=
  (ids.foldl (fun (acc : Std.HashSet Nat × Option Nat) i =>
    match acc with
    | (_, some d) => (acc.1, some d)
    | (seen, none) => if seen.contains i then (seen, some i) else (seen.insert i, none))
    (∅, none)).2

/-- Why a per-edge contract row cannot be attached to its edge, if it cannot:
    the edge does not exist, has no `source_override` (so its source keeps
    the node's own postconditions and the row would be ignored), starts at
    another node, or the row is not a postcondition. -/
def edgeRowProblem (edgeById : Std.HashMap Nat EdgeRow) (row : ContractRow) (role : ContractRole)
    (e : Nat) : Option String :=
  match edgeById.get? e with
  | none => some s!"contract row at {row.loc} names edge {e}, which does not exist"
  | some er =>
    if !er.sourceOverride then
      some s!"contract row at {row.loc} names edge {e}, which has no source_override"
    else if er.sourceId != row.nodeId then
      some s!"contract row at {row.loc} names node {row.nodeId}, but edge {e} starts at node {er.sourceId}"
    else if role != .postcondition then
      some s!"contract row at {row.loc} is a precondition of edge {e}; per-edge rows are postconditions"
    else none

/-- Translate raw database rows into a ContractGraph (pure), or say which row
    cannot be translated.

    Nodes carry the node contracts (rows with `edge_id` NULL), their
    definition locations and call-site flags. Range bounds are scaled by
    10^`rangeScale contractRows`. Each edge gets its own copies of its endpoints:
    - `source`: when `source_override`, postconditions := the translated rows
      whose `edge_id` is this edge (possibly none); otherwise the node itself.
    - `target`: when `target_param = p`, preconditions filtered to
      `appliesToParam p`; otherwise the node itself.
    Rows with `edge_id` set never belong to a node. Edges carry their site.

    Every row is translated or the result is an error, so no row is dropped:
    two nodes or two edges with one id, a node kind outside `nodeKinds`, an
    unknown enum string (`constraint_type`, `verification_level`,
    `contract_role`, `relationship`), a `malformed` contract row, a contract
    row whose `node_id` names no node, a per-edge row that `edgeRowProblem`
    rejects, and an edge whose endpoint names no node are errors. -/
def translateRows
    (nodeRows : List NodeRow)
    (contractRows : List ContractRow)
    (edgeRows : List EdgeRow)
    : Except String ContractGraph := do
  if let some id := firstDuplicate (nodeRows.map (·.id)) then
    throw s!"two nodes have id {id}"
  if let some id := firstDuplicate (edgeRows.map (·.id)) then
    throw s!"two edges have id {id}"
  if let some nr := nodeRows.find? (fun nr => !nodeKinds.contains nr.kind) then
    throw s!"node {nr.id} ({nr.name}) has unknown kind '{nr.kind}'"
  let scale := rangeScale contractRows
  let translated ← contractRows.mapM fun row => do
    return (row, ← row.contractRole, ← translateContractRow row scale)
  let nodeIds : Std.HashSet Nat := nodeRows.foldl (fun s nr => s.insert nr.id) ∅
  let edgeById : Std.HashMap Nat EdgeRow := edgeRows.foldl (fun m er => m.insert er.id er) ∅
  for (row, role, _) in translated do
    match row.edgeId with
    | none =>
      unless nodeIds.contains row.nodeId do
        throw s!"contract row at {row.loc} names node {row.nodeId}, which does not exist"
    | some e =>
      if let some msg := edgeRowProblem edgeById row role e then throw msg
  let byNode := groupBy (fun (r, _, _) => if r.edgeId.isNone then some r.nodeId else none) translated
  let byEdge := groupBy (fun (r, _, _) => r.edgeId) translated
  let withRole (role : ContractRole) (rows : List (ContractRow × ContractRole × List Constraint)) :=
    rows.flatMap fun (_, r, cs) => if r == role then cs else []
  let nodes := nodeRows.map fun nr =>
    let own := byNode.getD nr.id []
    ({ id := nr.id, name := nr.name, kind := nr.kind,
       preconditions := withRole .precondition own,
       postconditions := withRole .postcondition own,
       sourceFile := nr.sourceFile, sourceLine := nr.sourceLine,
       isCallSite := nr.isCallSite } : Node)
  let nodeById : Std.HashMap Nat Node := nodes.foldl (fun m n => m.insert n.id n) ∅
  let edges ← edgeRows.mapM fun row => do
    let some rel := parseRelationship row.relationship
      | throw s!"edge {row.id} has unknown relationship '{row.relationship}'"
    let some src := nodeById.get? row.sourceId
      | throw s!"edge {row.id} starts at node {row.sourceId}, which does not exist"
    let some tgt := nodeById.get? row.targetId
      | throw s!"edge {row.id} ends at node {row.targetId}, which does not exist"
    let src := if row.sourceOverride then
      { src with postconditions := (byEdge.getD row.id []).flatMap (·.2.2) }
    else src
    let tgt := match row.targetParam with
      | some p => { tgt with preconditions := tgt.preconditions.filter (appliesToParam p) }
      | none => tgt
    return ({ source := src, target := tgt, relationship := rel,
              siteFile := row.siteFile, siteLine := row.siteLine } : Edge)
  return { nodes := nodes, edges := edges }

/-- Read all nodes, contracts, and edges from the SQLite database (read-only)
    and translate them with `translateRows`, the only step that is not IO.
    Throws when `translateRows` rejects a row: a row that cannot be
    translated is an error, not an absent row. -/
def readContractGraph (dbPath : String) : IO ContractGraph := do
  let db ← openWith dbPath .readonly
  let nodeRows ← readNodes db
  let contractRows ← readContracts db
  let edgeRows ← readEdges db
  match translateRows nodeRows contractRows edgeRows with
  | .ok graph => return graph
  | .error msg => throw (IO.userError s!"cannot translate the database: {msg}")

end ContractGraph
