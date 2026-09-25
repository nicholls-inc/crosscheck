-- Translation.lean
--
-- SQLite → Lean proposition translation.
-- Uses leanprover/leansqlite low-level API to read the contract graph
-- from the SQLite database produced by Layer 1.

import ContractGraph.Types
import ContractGraph.DependentExpr
import SQLite

namespace ContractGraph

open SQLite

/-- Parse a constraint kind string from the database. -/
def parseConstraintKind (s : String) : ConstraintKind :=
  match s with
  | "precision"   => .precision
  | "nullability"  => .nullability
  | "type"         => .type
  | "range"        => .range
  | "length"       => .length
  | "choices"      => .choices
  | "range_min"    => .rangeMin  -- not a schema value; `range` rows carry lower bounds
  | _             => .precision  -- fallback; should not occur with valid schema

/-- Parse a verification level string from the database. -/
def parseVerifLevel (s : String) : VerificationLevel :=
  match s with
  | "PROVED"    => .proved
  | "TESTED"    => .tested
  | "EXTRACTED" => .extracted
  | "ASSUMED"   => .assumed
  | _           => .extracted  -- fallback

/-- Parse a contract role string from the database. -/
def parseContractRole (s : String) : ContractRole :=
  match s with
  | "precondition"  => .precondition
  | "postcondition" => .postcondition
  | _               => .precondition  -- fallback

/-- Parse a relationship string from the database. -/
def parseRelationship (s : String) : Relationship :=
  match s with
  | "calls"     => .calls
  | "writes_to" => .writesTo
  | "flows_to"  => .flowsTo
  | _           => .calls  -- fallback

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

/-- One `contracts` row, with the columns the checker reads. -/
structure ContractRow where
  nodeId            : Nat
  constraintType    : String
  decimalPlaces     : Option Int := none
  maxLength         : Option Int := none
  nullable          : Option Int := none
  typeName          : Option String := none
  minValue          : Option Int := none
  maxValue          : Option Int := none
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

/-- One `edges` row. -/
structure EdgeRow where
  id             : Nat
  sourceId       : Nat
  targetId       : Nat
  relationship   : Relationship
  /-- `some p`: only the target's preconditions with subject `p` (or none) apply. -/
  targetParam    : Option String := none
  /-- The source's postconditions are replaced by the rows with this edge's id. -/
  sourceOverride : Bool := false
  deriving Repr

/-- Translate a contracts row into Lean constraints (pure).

    Translation rules (SQL row → Lean Constraint):
    - precision:   staticBound ← param_decimal_places
    - nullability: staticBound ← param_nullable (0=NOT NULL, 1=NULL)
    - type:        typeName ← param_type_name
    - range:       up to two constraints: `range` (upper) with
                   staticBound ← param_max_value, and `rangeMin` (lower) with
                   staticBound ← param_min_value. A row with neither yields
                   one `range` constraint without a bound.
    - length:      staticBound ← param_max_length
    - choices:     choicesList ← param_choices.splitOn(",")
    `dependent_expr` goes on the upper `range` constraint, or on `rangeMin`
    when the row has only a lower bound. `subject` is copied to every
    constraint. -/
def translateContractRow (row : ContractRow) : List Constraint :=
  let kind := parseConstraintKind row.constraintType
  let depExpr := row.dependentExpr.bind parseDepExpr
  let base : Constraint := {
    kind := kind
    depExpr := depExpr
    sourceFile := row.sourceFile
    sourceLine := row.sourceLine
    verificationLevel := parseVerifLevel row.verificationLevel
    subject := row.subject
  }
  match kind with
  | .precision => [{ base with staticBound := row.decimalPlaces }]
  | .nullability => [{ base with staticBound := row.nullable }]
  | .length => [{ base with staticBound := row.maxLength }]
  | .type => [{ base with typeName := row.typeName }]
  | .choices => [{ base with choicesList := row.choices.map (·.splitOn ",") }]
  | .rangeMin => [{ base with staticBound := row.minValue }]
  | .range =>
    match row.minValue, row.maxValue with
    | some lo, some hi =>
      [{ base with staticBound := some hi },
       { base with kind := .rangeMin, staticBound := some lo, depExpr := none }]
    | some lo, none => [{ base with kind := .rangeMin, staticBound := some lo }]
    | none, hi => [{ base with staticBound := hi }]

/-- The role of a contracts row (`contract_role` NULL counts as precondition). -/
def ContractRow.contractRole (row : ContractRow) : ContractRole :=
  match row.role with
  | some r => parseContractRole r
  | none => .precondition

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
      17: subject, 18: edge_id -/
def readContracts (db : SQLite) : IO (List ContractRow) := do
  let subjectCol ← columnOrNull db "contracts" "subject"
  let edgeIdCol ← columnOrNull db "contracts" "edge_id"
  let stmt ← prepare db
    ("SELECT id, node_id, constraint_type, param_max_digits, param_decimal_places, " ++
     "param_max_length, param_nullable, param_type_name, param_min_value, param_max_value, " ++
     "param_choices, source_file, source_line, is_implicit, verification_level, " ++
     s!"contract_role, dependent_expr, {subjectCol}, {edgeIdCol} FROM contracts ORDER BY id")
  let mut results : List ContractRow := []
  let mut hasRow ← stmt.step
  while hasRow do
    let nodeId ← stmt.columnInt64 1
    let row : ContractRow := {
      nodeId := nodeId.toInt.toNat
      constraintType := ← stmt.columnText 2
      decimalPlaces := ← readOptionalInt stmt 4
      maxLength := ← readOptionalInt stmt 5
      nullable := ← readOptionalInt stmt 6
      typeName := ← readOptionalString stmt 7
      minValue := ← readOptionalInt stmt 8
      maxValue := ← readOptionalInt stmt 9
      choices := ← readOptionalString stmt 10
      sourceFile := ← stmt.columnText 11
      sourceLine := (← stmt.columnInt64 12).toInt.toNat
      verificationLevel := ← stmt.columnText 14
      role := ← readOptionalString stmt 15
      dependentExpr := ← readOptionalString stmt 16
      subject := ← readOptionalString stmt 17
      edgeId := (← readOptionalInt stmt 18).map Int.toNat
    }
    results := results ++ [row]
    hasRow ← stmt.step
  return results

/-- Read all nodes from the database. Returns (id, name, kind) triples.
    `qualified_name` is not read. -/
def readNodes (db : SQLite) : IO (List (Nat × String × String)) := do
  let stmt ← prepare db
    "SELECT id, name, kind FROM nodes ORDER BY id"
  let mut results : List (Nat × String × String) := []
  let mut hasRow ← stmt.step
  while hasRow do
    let nodeId ← stmt.columnInt64 0
    let name ← stmt.columnText 1
    let kind ← stmt.columnText 2
    results := results ++ [(nodeId.toInt.toNat, name, kind)]
    hasRow ← stmt.step
  return results

/-- Read all edges from the database. -/
def readEdges (db : SQLite) : IO (List EdgeRow) := do
  let targetParamCol ← columnOrNull db "edges" "target_param"
  let overrideCol ← columnOrNull db "edges" "source_override"
  let stmt ← prepare db
    (s!"SELECT id, source_node_id, target_node_id, relationship, {targetParamCol}, " ++
     s!"{overrideCol} FROM edges ORDER BY id")
  let mut results : List EdgeRow := []
  let mut hasRow ← stmt.step
  while hasRow do
    let edgeId ← stmt.columnInt64 0
    let srcId ← stmt.columnInt64 1
    let tgtId ← stmt.columnInt64 2
    let rel ← stmt.columnText 3
    let targetParam ← readOptionalString stmt 4
    let override ← readOptionalInt stmt 5
    results := results ++ [{
      id := edgeId.toInt.toNat
      sourceId := srcId.toInt.toNat
      targetId := tgtId.toInt.toNat
      relationship := parseRelationship rel
      targetParam := targetParam
      sourceOverride := override.getD 0 != 0 }]
    hasRow ← stmt.step
  return results

/-- Whether a precondition applies to the argument bound to `param`:
    its subject is that parameter, or it has no subject (all parameters). -/
def appliesToParam (param : String) (c : Constraint) : Bool :=
  match c.subject with
  | none => true
  | some s => s == param

/-- Build a ContractGraph from raw database rows (pure).

    Nodes carry the node contracts (rows with `edge_id` NULL). Each edge gets
    its own copies of its endpoints:
    - `source`: when `source_override`, postconditions := the translated rows
      whose `edge_id` is this edge (possibly none); otherwise the node itself.
    - `target`: when `target_param = p`, preconditions filtered to
      `appliesToParam p`; otherwise the node itself.
    Rows with `edge_id` set never belong to a node. -/
def buildGraph
    (nodeRows : List (Nat × String × String))
    (contractRows : List ContractRow)
    (edgeRows : List EdgeRow)
    : ContractGraph :=
  let nodeContracts := contractRows.filter (·.edgeId.isNone)
  let nodes := nodeRows.map fun (id, name, kind) =>
    let own := nodeContracts.filter (·.nodeId == id)
    let preconditions := own.flatMap fun row =>
      match row.contractRole with
      | .precondition => translateContractRow row
      | .postcondition => []
    let postconditions := own.flatMap fun row =>
      match row.contractRole with
      | .postcondition => translateContractRow row
      | .precondition => []
    ({ id := id, name := name, kind := kind,
       preconditions := preconditions, postconditions := postconditions } : Node)

  let findNode (nid : Nat) : Option Node :=
    nodes.find? (fun n => n.id == nid)

  let edges := edgeRows.filterMap fun row =>
    match findNode row.sourceId, findNode row.targetId with
    | some src, some tgt =>
      let src := if row.sourceOverride then
        { src with postconditions :=
            (contractRows.filter (·.edgeId == some row.id)).flatMap translateContractRow }
      else src
      let tgt := match row.targetParam with
        | some p => { tgt with preconditions := tgt.preconditions.filter (appliesToParam p) }
        | none => tgt
      some ({ source := src, target := tgt, relationship := row.relationship } : Edge)
    | _, _ => none

  { nodes := nodes, edges := edges }

/-- Read all nodes, contracts, and edges from the SQLite database.
    Uses leanprover/leansqlite low-level API.

    Implementation:
    1. Open the database in read-only mode
    2. Read all rows from nodes, contracts, edges tables
    3. Group node contracts by node_id and contract_role; attach per-edge
       rows and target_param filtering to each edge's endpoint copies
    4. Construct the typed ContractGraph structure -/
def readContractGraph (dbPath : String) : IO ContractGraph := do
  let db ← openWith dbPath .readonly
  let nodeRows ← readNodes db
  let contractRows ← readContracts db
  let edgeRows ← readEdges db
  return buildGraph nodeRows contractRows edgeRows

end ContractGraph
