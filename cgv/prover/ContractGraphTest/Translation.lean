-- Translation.lean
-- `translateRows` (TB-1.12): every row is translated or the rows are
-- rejected, so the checker never runs on a graph that silently lacks a row.
-- One `#guard` per rejection, each checking the reason, and the accept cases.

import ContractGraph.Translation

namespace ContractGraphTest.Translation

open ContractGraph

instance : Inhabited ContractGraph := ⟨{ nodes := [], edges := [] }⟩

/-- `translateRows` on rows a test means to be well formed. A rejection
    panics, which `lake build` prints but does not fail on; build with
    `LEAN_ABORT_ON_PANIC=1` to make it fail. -/
def graphOf (nodeRows : List NodeRow) (contractRows : List ContractRow)
    (edgeRows : List EdgeRow) : ContractGraph :=
  match translateRows nodeRows contractRows edgeRows with
  | .ok g => g
  | .error msg => panic! s!"translateRows rejected test rows: {msg}"

/-- `make` (4dp) writes `Invoice.total` (2dp) on edge 1, whose override row
    says 2dp. -/
def nodes : List NodeRow := [(1, "make", "function"), (2, "Invoice.total", "model")]

def ownPost : ContractRow :=
  { nodeId := 1, constraintType := "precision", decimalPlaces := some 4,
    role := some "postcondition", sourceFile := "m.py", sourceLine := 1 }
def fieldPre : ContractRow :=
  { nodeId := 2, constraintType := "precision", decimalPlaces := some 2,
    role := some "precondition", sourceFile := "models.py", sourceLine := 5 }
def edgePost : ContractRow :=
  { nodeId := 1, constraintType := "precision", decimalPlaces := some 2,
    role := some "postcondition", edgeId := some 1, sourceFile := "m.py", sourceLine := 3 }
def contracts : List ContractRow := [ownPost, fieldPre, edgePost]

def write : EdgeRow :=
  { id := 1, sourceId := 1, targetId := 2, relationship := "writes_to", sourceOverride := true }
def edges : List EdgeRow := [write]

/-- Whether `translateRows` rejects the rows with a message containing `reason`. -/
def rejects (n : List NodeRow) (c : List ContractRow) (e : List EdgeRow) (reason : String) : Bool :=
  match translateRows n c e with
  | .error msg => (msg.splitOn reason).length > 1
  | .ok _ => false

def accepts (n : List NodeRow) (c : List ContractRow) (e : List EdgeRow) : Bool :=
  (translateRows n c e).isOk

/-! ## Accepted -/

#guard accepts nodes contracts edges
-- Each node and edge row gives one node and one edge; the edge's source copy
-- carries the override row only.
#guard (translateRows nodes contracts edges).toOption.map (fun g =>
    (g.nodes.map (fun n => (n.id, n.preconditions.length, n.postconditions.length)),
     g.edges.map (fun e => (e.source.postconditions.map (·.staticBound), e.relationship == .writesTo))))
  == some ([(1, 0, 1), (2, 1, 0)], [([some 2], true)])
-- A NULL `contract_role` is a precondition.
#guard (translateRows nodes [{ fieldPre with role := none }] []).toOption.map
    (fun g => g.nodes.map (·.preconditions.length)) == some [0, 1]
-- `range_min` is not a schema value, but stays accepted as a lower bound.
def rangeMinRow : ContractRow :=
  { fieldPre with constraintType := "range_min", decimalPlaces := none, minValue := some 0 }
#guard (translateRows nodes [rangeMinRow] []).toOption.map
    (fun g => g.nodes.map (·.preconditions.map (·.kind))) == some [[], [.rangeMin]]
-- A bound kind with only a dependent expression is a constraint.
def depRow (e : String) : ContractRow :=
  { ownPost with decimalPlaces := none, dependentExpr := some e }
#guard accepts nodes [depRow "max(input_precision, 2)"] []
-- Each kind with its value.
#guard accepts nodes
  [{ fieldPre with constraintType := "length", decimalPlaces := none, maxLength := some 5 },
   { fieldPre with constraintType := "nullability", decimalPlaces := none, nullable := some 0 },
   { fieldPre with constraintType := "type", decimalPlaces := none, typeName := some "str" },
   { fieldPre with constraintType := "choices", decimalPlaces := none, choices := some "[\"a\"]" },
   { fieldPre with constraintType := "range", decimalPlaces := none, maxDecimal := some "9.5" }] []

/-! ## Unknown enum strings -/

#guard rejects nodes [{ fieldPre with constraintType := "decimal_places" }] [] "unknown constraint_type"
#guard rejects nodes [{ fieldPre with verificationLevel := "GUESSED" }] []
  "unknown verification_level 'GUESSED'"
#guard rejects nodes [{ fieldPre with role := some "guarantee" }] [] "unknown contract_role 'guarantee'"
#guard rejects nodes [{ fieldPre with role := some "" }] [] "unknown contract_role ''"
#guard rejects nodes contracts [{ write with relationship := "writes" }] "unknown relationship 'writes'"
#guard rejects [(1, "make", "function"), (2, "Invoice.total", "Model")] contracts edges
  "unknown kind 'Model'"

/-! ## Rows that name no row -/

#guard rejects nodes [ownPost, fieldPre] [{ write with sourceId := 9 }] "edge 1 starts at node 9, which does not exist"
#guard rejects nodes [ownPost, fieldPre] [{ write with targetId := 9 }] "edge 1 ends at node 9, which does not exist"
#guard rejects nodes [{ fieldPre with nodeId := 9 }] [] "names node 9, which does not exist"
#guard rejects nodes [{ edgePost with edgeId := some 9 }] edges "names edge 9, which does not exist"
#guard rejects nodes contracts [{ write with sourceOverride := false }] "has no source_override"
#guard rejects nodes [{ edgePost with nodeId := 2 }] edges "but edge 1 starts at node 1"
#guard rejects nodes [{ edgePost with role := some "precondition" }] edges
  "per-edge rows are postconditions"
#guard rejects (nodes ++ [((1, "again", "function") : NodeRow)]) contracts edges "two nodes have id 1"
#guard rejects nodes contracts (edges ++ [write]) "two edges have id 1"

/-! ## Rows without their value -/

#guard rejects nodes [{ fieldPre with decimalPlaces := none }] [] "no value and no dependent_expr"
#guard ["length", "nullability", "type", "choices", "range", "range_min"].all fun k =>
  rejects nodes [{ fieldPre with constraintType := k, decimalPlaces := none }] []
    "no value and no dependent_expr"
-- A `range_min` row reads only the lower bound.
#guard rejects nodes [{ rangeMinRow with minValue := none, maxValue := some 5 }] []
  "no value and no dependent_expr"
#guard rejects nodes [depRow "max(input_precision"] [] "malformed dependent_expr"

end ContractGraphTest.Translation
