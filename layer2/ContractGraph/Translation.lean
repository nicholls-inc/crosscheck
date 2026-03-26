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
  | _              => .precision  -- fallback; should not occur with valid schema

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

/-- Translate a single contracts row into a Constraint.

    contracts table column layout (0-indexed):
      0: id, 1: node_id, 2: constraint_type,
      3: param_max_digits, 4: param_decimal_places,
      5: param_max_length, 6: param_nullable,
      7: param_type_name, 8: param_min_value,
      9: param_max_value, 10: param_choices,
      11: source_file, 12: source_line,
      13: is_implicit, 14: verification_level,
      15: contract_role, 16: dependent_expr -/
private def translateContractRow (stmt : Stmt)
    : IO (Nat × ContractRole × Constraint) := do
  let nodeId ← stmt.columnInt64 1
  let constraintType ← stmt.columnText 2
  let paramDecimalPlaces ← readOptionalInt stmt 4
  let paramMaxLength ← readOptionalInt stmt 5
  let paramNullable ← readOptionalInt stmt 6
  let paramTypeName ← readOptionalString stmt 7
  let _paramMinValue ← readOptionalInt stmt 8
  let paramMaxValue ← readOptionalInt stmt 9
  let paramChoices ← readOptionalString stmt 10
  let sourceFile ← stmt.columnText 11
  let sourceLine ← stmt.columnInt64 12
  let verifLevel ← stmt.columnText 14
  let contractRoleStr ← readOptionalString stmt 15
  let depExprStr ← readOptionalString stmt 16

  let kind := parseConstraintKind constraintType
  let role := match contractRoleStr with
    | some r => parseContractRole r
    | none => .precondition

  -- Determine staticBound based on constraint kind
  let staticBound : Option Int := match kind with
    | .precision => paramDecimalPlaces
    | .nullability => paramNullable
    | .length => paramMaxLength
    | .range => paramMaxValue  -- use max_value as upper bound
    | _ => none

  -- Parse dependent expression if present
  let depExpr := match depExprStr with
    | some s => parseDepExpr s
    | none => none

  -- Parse type name for type constraints
  let typeName := match kind with
    | .type => paramTypeName
    | _ => none

  -- Parse choices list
  let choicesList := match kind with
    | .choices => paramChoices.map (·.splitOn ",")
    | _ => none

  let constraint : Constraint := {
    kind := kind
    staticBound := staticBound
    depExpr := depExpr
    typeName := typeName
    choicesList := choicesList
    sourceFile := sourceFile
    sourceLine := sourceLine.toInt.toNat
    verificationLevel := parseVerifLevel verifLevel
  }

  return (nodeId.toInt.toNat, role, constraint)

/-- Read all contracts from the database and group by node ID.
    Returns (nodeId, contractRole, constraint) triples.

    Translation rules (SQL row → Lean Constraint):
    - precision:   staticBound ← param_decimal_places
    - nullability: staticBound ← param_nullable (0=NOT NULL, 1=NULL)
    - type:        typeName ← param_type_name
    - range:       staticBound ← param_max_value
    - length:      staticBound ← param_max_length
    - choices:     choicesList ← param_choices.splitOn(",") -/
def readContracts (db : SQLite) : IO (List (Nat × ContractRole × Constraint)) := do
  let stmt ← prepare db
    "SELECT id, node_id, constraint_type, param_max_digits, param_decimal_places, param_max_length, param_nullable, param_type_name, param_min_value, param_max_value, param_choices, source_file, source_line, is_implicit, verification_level, contract_role, dependent_expr FROM contracts ORDER BY id"
  let mut results : List (Nat × ContractRole × Constraint) := []
  let mut hasRow ← stmt.step
  while hasRow do
    let row ← translateContractRow stmt
    results := results ++ [row]
    hasRow ← stmt.step
  return results

/-- Read all nodes from the database. Returns (id, name, kind) triples. -/
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

/-- Read all edges from the database.
    Returns (sourceNodeId, targetNodeId, relationship) triples. -/
def readEdges (db : SQLite) : IO (List (Nat × Nat × Relationship)) := do
  let stmt ← prepare db
    "SELECT id, source_node_id, target_node_id, relationship FROM edges ORDER BY id"
  let mut results : List (Nat × Nat × Relationship) := []
  let mut hasRow ← stmt.step
  while hasRow do
    let srcId ← stmt.columnInt64 1
    let tgtId ← stmt.columnInt64 2
    let rel ← stmt.columnText 3
    results := results ++ [(srcId.toInt.toNat, tgtId.toInt.toNat, parseRelationship rel)]
    hasRow ← stmt.step
  return results

/-- Build a ContractGraph from raw database rows. -/
private def buildGraph
    (nodeRows : List (Nat × String × String))
    (contractRows : List (Nat × ContractRole × Constraint))
    (edgeRows : List (Nat × Nat × Relationship))
    : ContractGraph :=
  -- Build nodes with their contracts grouped by role
  let nodes := nodeRows.map fun (id, name, kind) =>
    let nodeContracts := contractRows.filter (fun (nid, _, _) => nid == id)
    let preconditions := nodeContracts.filterMap fun (_, role, c) =>
      match role with
      | .precondition => some c
      | .postcondition => none
    let postconditions := nodeContracts.filterMap fun (_, role, c) =>
      match role with
      | .postcondition => some c
      | .precondition => none
    ({ id := id, name := name, kind := kind,
       preconditions := preconditions, postconditions := postconditions } : Node)

  -- Build a lookup from node ID to Node
  let findNode (nid : Nat) : Option Node :=
    nodes.find? (fun n => n.id == nid)

  -- Build edges
  let edges := edgeRows.filterMap fun (srcId, tgtId, rel) =>
    match findNode srcId, findNode tgtId with
    | some src, some tgt =>
      some ({ source := src, target := tgt, relationship := rel } : Edge)
    | _, _ => none

  { nodes := nodes, edges := edges }

/-- Read all nodes, contracts, and edges from the SQLite database.
    Uses leanprover/leansqlite low-level API.

    Implementation:
    1. Open the database in read-only mode
    2. Read all rows from nodes, contracts, edges tables
    3. Group contracts by node_id and contract_role
    4. Construct the typed ContractGraph structure -/
def readContractGraph (dbPath : String) : IO ContractGraph := do
  let db ← openWith dbPath .readonly
  let nodeRows ← readNodes db
  let contractRows ← readContracts db
  let edgeRows ← readEdges db
  return buildGraph nodeRows contractRows edgeRows

end ContractGraph
