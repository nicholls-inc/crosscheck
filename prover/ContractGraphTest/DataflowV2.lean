-- DataflowV2.lean
-- Checker behaviour of data-flow model v2 (docs/design/dataflow-v2.md):
-- lower bounds, per-edge source overrides, target_param filtering,
-- composition through the next edge's copy of a node, `calls` edges not
-- followed, hop attribution, and the every-hop missing-postcondition warning.
-- Graphs are built with `buildGraph` from row structures, so the translation
-- rules for `edge_id`, `source_override`, `target_param` and `subject` are
-- exercised too.

import ContractGraph.Types
import ContractGraph.Checker
import ContractGraph.Composition
import ContractGraph.Diagnostics
import ContractGraph.Translation
import ContractGraph.Main

namespace ContractGraphTest.DataflowV2

open ContractGraph

instance : Inhabited Node := ⟨{ id := 0, name := "", kind := "", preconditions := [],
                                postconditions := [] }⟩
instance : Inhabited Edge := ⟨{ source := default, target := default, relationship := .calls }⟩

/-- A constraint with a static bound. -/
def bound (k : ConstraintKind) (b : Int) (line : Nat := 1) : Constraint :=
  { kind := k, staticBound := some b, sourceFile := "t.py", sourceLine := line,
    verificationLevel := .extracted }

def isConsistent : CheckResult → Bool
  | .consistent => true
  | .inconsistent _ => false

def contains (s sub : String) : Bool := (s.splitOn sub).length > 1

def errors (o : CheckOutput) : List ResultEntry := o.results.filter (·.severity == "error")
def warnings (o : CheckOutput) : List ResultEntry := o.results.filter (·.severity == "warning")

/-! ## 1. Lower bounds (`rangeMin`) -/

-- Source guarantees ≥ -50, target requires ≥ 0: inconsistent.
#guard !isConsistent (checkConstraintPair (bound .rangeMin (-50)) (bound .rangeMin 0))
-- Source guarantees ≥ 5, target requires ≥ 0: consistent.
#guard isConsistent (checkConstraintPair (bound .rangeMin 5) (bound .rangeMin 0))
-- Equal bounds are consistent.
#guard isConsistent (checkConstraintPair (bound .rangeMin 0) (bound .rangeMin 0))
-- Lower and upper bounds do not interact.
#guard isConsistent (checkConstraintPair (bound .rangeMin (-50)) (bound .range 0))
#guard isConsistent (checkConstraintPair (bound .range 100) (bound .rangeMin 0))

-- Range bounds are held in micros (value × 10^6) and displayed as decimals.
#guard formatBound (bound .rangeMin (-50000000)) == "range ≥ -50"
#guard formatBound (bound .range 10000000) == "range ≤ 10"
#guard toString ConstraintKind.rangeMin == "range_min"

-- A `range` row with both bounds yields an upper and a lower constraint.
def rangeRow (lo hi : Option Int) : ContractRow :=
  { nodeId := 1, constraintType := "range", minValue := lo, maxValue := hi }

def kindsAndBounds (cs : List Constraint) : List (ConstraintKind × Option Int) :=
  cs.map fun c => (c.kind, c.staticBound)

-- (`minValue`/`maxValue` are plain units; the constraints are in micros.)
#guard kindsAndBounds (translateContractRow (rangeRow (some 0) (some 10)))
  == [(.range, some 10000000), (.rangeMin, some 0)]
#guard kindsAndBounds (translateContractRow (rangeRow (some (-5)) none))
  == [(.rangeMin, some (-5000000))]
#guard kindsAndBounds (translateContractRow (rangeRow none (some 7)))
  == [(.range, some 7000000)]

-- The soundness theorem covers lower bounds: a consistent pair gives tr ≤ sg.
example : constraintImplies (bound .rangeMin 5) (bound .rangeMin 0) :=
  pair_sound _ _ rfl (by rfl)

/-- adjust (range ≥ -50) writes_to Stock.qty (PositiveIntegerField: range ≥ 0). -/
def lowerGraph : ContractGraph :=
  buildGraph
    [(1, "adjust", "function"), (2, "Stock.qty", "model")]
    [{ nodeId := 1, constraintType := "range", minValue := some (-50),
       role := some "postcondition", sourceFile := "inv.py", sourceLine := 4 },
     { nodeId := 2, constraintType := "range", minValue := some 0,
       role := some "precondition", sourceFile := "models.py", sourceLine := 9 }]
    [{ id := 1, sourceId := 1, targetId := 2, relationship := .writesTo }]

#guard (errors (runChecker lowerGraph)).map (fun r => (r.sourceGuarantee, r.targetRequirement))
  == [("range ≥ -50", "range ≥ 0")]

/-! ## 2. Per-edge source override

`make` writes a 2dp value to `Invoice.total` and a 4dp value to
`Invoice.fee`; both fields are DecimalField(decimal_places=2). The function's
own postconditions (6dp for its return value) are not used on either write. -/

def overrideGraph : ContractGraph :=
  buildGraph
    [(1, "make", "function"), (2, "Invoice.total", "model"), (3, "Invoice.fee", "model")]
    [ -- make's return value: 6dp (must not be applied to the writes)
      { nodeId := 1, constraintType := "precision", decimalPlaces := some 6,
        role := some "postcondition", sourceLine := 1 },
      { nodeId := 2, constraintType := "precision", decimalPlaces := some 2,
        role := some "precondition", sourceLine := 10 },
      { nodeId := 3, constraintType := "precision", decimalPlaces := some 2,
        role := some "precondition", sourceLine := 11 },
      -- override rows: 2dp on edge 1, 4dp on edge 2
      { nodeId := 1, constraintType := "precision", decimalPlaces := some 2,
        role := some "postcondition", edgeId := some 1, sourceLine := 5 },
      { nodeId := 1, constraintType := "precision", decimalPlaces := some 4,
        role := some "postcondition", edgeId := some 2, sourceLine := 6 } ]
    [ { id := 1, sourceId := 1, targetId := 2, relationship := .writesTo, sourceOverride := true },
      { id := 2, sourceId := 1, targetId := 3, relationship := .writesTo, sourceOverride := true } ]

-- Override rows never become node contracts.
#guard (overrideGraph.nodes.map (·.postconditions.length)) == [1, 0, 0]
-- Each edge's source copy carries only that edge's rows.
#guard overrideGraph.edges.map (fun e => e.source.postconditions.map (·.staticBound))
  == [[some 2], [some 4]]
#guard (checkEdgeAllFull (overrideGraph.edges[0]!)).all isConsistent
#guard !(checkEdgeAllFull (overrideGraph.edges[1]!)).all isConsistent
#guard (errors (runChecker overrideGraph)).map (fun r => (r.path, r.sourceGuarantee, r.source.line))
  == [(["make", "Invoice.fee"], "precision ≤ 4", 6)]

-- An override with no rows: the source guarantees nothing, and the field's
-- precision requirement passes vacuously with a warning.
def emptyOverrideGraph : ContractGraph :=
  buildGraph
    [(1, "make", "function"), (2, "Invoice.total", "model")]
    [{ nodeId := 1, constraintType := "precision", decimalPlaces := some 6,
       role := some "postcondition" },
     { nodeId := 2, constraintType := "precision", decimalPlaces := some 2,
       role := some "precondition" }]
    [{ id := 1, sourceId := 1, targetId := 2, relationship := .writesTo, sourceOverride := true }]

#guard (errors (runChecker emptyOverrideGraph)).isEmpty
#guard (warnings (runChecker emptyOverrideGraph)).map (·.hop) == [["make", "Invoice.total"]]
#guard (runChecker emptyOverrideGraph).exitCode == 0

-- Without the override flag, rows with an edge_id are ignored and the node's
-- own 6dp postcondition applies.
def noOverrideGraph : ContractGraph :=
  buildGraph
    [(1, "make", "function"), (2, "Invoice.total", "model")]
    [{ nodeId := 1, constraintType := "precision", decimalPlaces := some 6,
       role := some "postcondition" },
     { nodeId := 1, constraintType := "precision", decimalPlaces := some 2,
       role := some "postcondition", edgeId := some 1 },
     { nodeId := 2, constraintType := "precision", decimalPlaces := some 2,
       role := some "precondition" }]
    [{ id := 1, sourceId := 1, targetId := 2, relationship := .writesTo }]

#guard (errors (runChecker noOverrideGraph)).map (·.sourceGuarantee) == ["precision ≤ 6"]

/-! ## 3. target_param filtering and 6. hop attribution

`combine(amount, rate)` requires precision(amount) ≤ 2 and precision(rate) ≤ 6,
and non-null for every parameter (subject NULL). `rate_of` (4dp) is passed as
`rate`; `with_tax` (4dp) is passed as `amount`. -/

def paramGraph : ContractGraph :=
  buildGraph
    [(1, "rate_of", "function"), (2, "with_tax", "function"),
     (3, "combine", "function"), (4, "Invoice.total", "model")]
    [ { nodeId := 1, constraintType := "precision", decimalPlaces := some 4,
        role := some "postcondition", sourceFile := "p.py", sourceLine := 1 },
      { nodeId := 1, constraintType := "nullability", nullable := some 0,
        role := some "postcondition", sourceFile := "p.py", sourceLine := 1 },
      { nodeId := 2, constraintType := "precision", decimalPlaces := some 4,
        role := some "postcondition", sourceFile := "p.py", sourceLine := 2 },
      { nodeId := 2, constraintType := "nullability", nullable := some 0,
        role := some "postcondition", sourceFile := "p.py", sourceLine := 2 },
      { nodeId := 3, constraintType := "precision", decimalPlaces := some 2,
        role := some "precondition", subject := some "amount",
        sourceFile := "p.py", sourceLine := 3 },
      { nodeId := 3, constraintType := "precision", decimalPlaces := some 6,
        role := some "precondition", subject := some "rate",
        sourceFile := "p.py", sourceLine := 4 },
      { nodeId := 3, constraintType := "nullability", nullable := some 0,
        role := some "precondition", sourceFile := "p.py", sourceLine := 5 },
      { nodeId := 3, constraintType := "precision", decimalPlaces := some 2,
        role := some "postcondition", sourceFile := "p.py", sourceLine := 6 },
      { nodeId := 3, constraintType := "nullability", nullable := some 0,
        role := some "postcondition", sourceFile := "p.py", sourceLine := 6 },
      { nodeId := 4, constraintType := "precision", decimalPlaces := some 2,
        role := some "precondition", sourceFile := "m.py", sourceLine := 7 },
      { nodeId := 4, constraintType := "nullability", nullable := some 0,
        role := some "precondition", sourceFile := "m.py", sourceLine := 7 } ]
    [ { id := 1, sourceId := 1, targetId := 3, relationship := .flowsTo, targetParam := some "rate" },
      { id := 2, sourceId := 2, targetId := 3, relationship := .flowsTo, targetParam := some "amount" },
      { id := 3, sourceId := 3, targetId := 4, relationship := .writesTo } ]

-- The node keeps every precondition; each edge's target copy only the
-- bound parameter's and the subject-less ones.
#guard (paramGraph.nodes[2]!).preconditions.length == 3
#guard paramGraph.edges.map (fun e => e.target.preconditions.map (·.sourceLine))
  == [[4, 5], [3, 5], [7, 7]]
#guard (checkEdgeAllFull (paramGraph.edges[0]!)).all isConsistent
#guard !(checkEdgeAllFull (paramGraph.edges[1]!)).all isConsistent

-- Only with_tax's value violates its parameter's requirement. The finding's
-- target is the hop target `combine`, not the path end `Invoice.total`;
-- source stays the path head.
#guard (errors (runChecker paramGraph)).map
    (fun r => (r.path, r.source.name, r.target.name, r.hop, r.target.line))
  == [(["with_tax", "combine", "Invoice.total"], "with_tax", "combine",
       ["with_tax", "combine"], 3)]
#guard (warnings (runChecker paramGraph)).isEmpty
#guard contains (outputToJson (runChecker paramGraph)) "\"hop\": [\"with_tax\", \"combine\"]"

-- The hop is recorded on the diagnostic itself.
#guard (checkEdgeAllFull (paramGraph.edges[1]!)).filterMap (fun r =>
    match r with
    | .inconsistent d => some (d.hopSource, d.hopTarget)
    | .consistent => none)
  == [("with_tax", "combine")]

/-! ## 4. Composition through the next edge's override

`pick(p)` (one parameter) writes `p.quantize(0.001) if c else p` to
`Reading.kwh` (3dp): the override row is `max(3, input_precision)`. `pick`
has no return postconditions of its own. `measure` (4dp) flows into `pick`. -/

def depGraph : ContractGraph :=
  buildGraph
    [(1, "measure", "function"), (2, "pick", "function"), (3, "Reading.kwh", "model")]
    [ { nodeId := 1, constraintType := "precision", decimalPlaces := some 4,
        role := some "postcondition", sourceFile := "a.py", sourceLine := 1 },
      { nodeId := 2, constraintType := "precision", dependentExpr := some "max(3, input_precision)",
        role := some "postcondition", edgeId := some 2, sourceFile := "a.py", sourceLine := 8 },
      { nodeId := 3, constraintType := "precision", decimalPlaces := some 3,
        role := some "precondition", sourceFile := "m.py", sourceLine := 2 } ]
    [ { id := 1, sourceId := 1, targetId := 2, relationship := .flowsTo },
      { id := 2, sourceId := 2, targetId := 3, relationship := .writesTo, sourceOverride := true } ]

-- measure → pick → Reading.kwh: max(3, 4) = 4 > 3.
#guard (errors (runChecker depGraph)).map (fun r => (r.path, r.sourceGuarantee, r.hop))
  == [(["measure", "pick", "Reading.kwh"], "precision ≤ 4", ["pick", "Reading.kwh"])]
-- pick → Reading.kwh alone: the bound is unresolved, only a warning.
#guard (warnings (runChecker depGraph)).map (fun r => (r.path, contains r.suggestion "could not be resolved"))
  == [(["pick", "Reading.kwh"], true)]
#guard (checkPath [depGraph.edges[1]!]).all (fun r =>
    match r with
    | .consistent => true
    | .inconsistent d => d.severity == .warning)

-- Composing through `edge.target` (pick's own, empty, postconditions) would
-- miss it: the composed node has nothing to check.
#guard (composeContracts (depGraph.edges[0]!).source (depGraph.edges[0]!).target).isEmpty
#guard (composeContracts (depGraph.edges[0]!).source (depGraph.edges[1]!).source).map (·.staticBound)
  == [some 4]

-- With measure at 2dp, max(3, 2) = 3 ≤ 3: no error on the composed path.
def depGraphClean : ContractGraph :=
  { depGraph with
    edges := depGraph.edges.map fun e =>
      if e.source.name == "measure" then
        { e with source := { e.source with postconditions := [bound .precision 2] } }
      else e }

#guard (errors (runChecker depGraphClean)).isEmpty

/-! ## 5. `calls` edges are not followed -/

def callsGraph : ContractGraph :=
  buildGraph
    [(1, "caller", "function"), (2, "label", "function"), (3, "Invoice.customer", "model")]
    [ { nodeId := 2, constraintType := "length", maxLength := some 64,
        role := some "postcondition" },
      { nodeId := 3, constraintType := "length", maxLength := some 32,
        role := some "precondition" } ]
    [ { id := 1, sourceId := 1, targetId := 2, relationship := .calls },
      { id := 2, sourceId := 2, targetId := 3, relationship := .writesTo } ]

-- The calls edge stays in the graph ...
#guard callsGraph.edges.length == 2
-- ... but only the data path is enumerated.
#guard (enumeratePaths callsGraph).map (·.map (·.source.name)) == [["label"]]
#guard (errors (runChecker callsGraph)).map (·.path) == [["label", "Invoice.customer"]]

-- A graph whose only route to a model is through a calls edge has no paths.
def callsOnlyGraph : ContractGraph :=
  buildGraph
    [(1, "f", "function"), (2, "M.x", "model")]
    [{ nodeId := 1, constraintType := "precision", decimalPlaces := some 9,
       role := some "postcondition" },
     { nodeId := 2, constraintType := "precision", decimalPlaces := some 2,
       role := some "precondition" }]
    [{ id := 1, sourceId := 1, targetId := 2, relationship := .calls }]

#guard (enumeratePaths callsOnlyGraph).isEmpty
#guard (runChecker callsOnlyGraph).results.isEmpty

/-! ## 7. Final-hop (and single-edge) missing-postcondition warning

`make` guarantees only a type; the field requires precision ≤ 2, non-null,
range ≥ 0, a type and choices. The precision, nullability, rangeMin and
choices requirements pass vacuously and warn; type does not. -/

def finalHopGraph : ContractGraph :=
  buildGraph
    [(1, "make", "function"), (2, "Invoice.total", "model")]
    [ { nodeId := 1, constraintType := "type", typeName := some "Decimal",
        role := some "postcondition" },
      { nodeId := 2, constraintType := "precision", decimalPlaces := some 2,
        role := some "precondition", sourceLine := 1 },
      { nodeId := 2, constraintType := "nullability", nullable := some 0,
        role := some "precondition", sourceLine := 2 },
      { nodeId := 2, constraintType := "range", minValue := some 0,
        role := some "precondition", sourceLine := 3 },
      { nodeId := 2, constraintType := "type", typeName := some "Decimal",
        role := some "precondition", sourceLine := 4 },
      { nodeId := 2, constraintType := "choices", choices := some "a,b",
        role := some "precondition", sourceLine := 5 } ]
    [ { id := 1, sourceId := 1, targetId := 2, relationship := .writesTo } ]

#guard (warnings (runChecker finalHopGraph)).map
    (fun r => (r.target.line, r.hop, contains r.suggestion "pass vacuously",
               contains r.suggestion "'make'"))
  == [(1, ["make", "Invoice.total"], true, true),
      (2, ["make", "Invoice.total"], true, true),
      (3, ["make", "Invoice.total"], true, true),
      (5, ["make", "Invoice.total"], true, true)]
#guard (errors (runChecker finalHopGraph)).isEmpty
-- Warnings never affect the exit code.
#guard (runChecker finalHopGraph).exitCode == 0

-- On a two-hop path the last hop warns too, naming the (composed)
-- intermediate node as the hop source; the shorter path's copy is kept.
def twoHopGraph : ContractGraph :=
  buildGraph
    [(1, "outer", "function"), (2, "inner", "function"), (3, "Invoice.total", "model")]
    [ { nodeId := 1, constraintType := "type", typeName := some "Decimal",
        role := some "postcondition" },
      { nodeId := 2, constraintType := "type", typeName := some "Decimal",
        role := some "postcondition" },
      { nodeId := 3, constraintType := "precision", decimalPlaces := some 2,
        role := some "precondition" } ]
    [ { id := 1, sourceId := 1, targetId := 2, relationship := .flowsTo },
      { id := 2, sourceId := 2, targetId := 3, relationship := .writesTo } ]

-- Before deduplication the warning appears on both paths ...
#guard ((collectResults (checkAllPaths twoHopGraph)).filter (·.severity == "warning")).map
    (fun r => (r.path, r.hop))
  == [(["outer", "inner", "Invoice.total"], ["inner", "Invoice.total"]),
      (["inner", "Invoice.total"], ["inner", "Invoice.total"])]
-- ... and is reported once, under the shortest path.
#guard (warnings (runChecker twoHopGraph)).map (·.path) == [["inner", "Invoice.total"]]

end ContractGraphTest.DataflowV2
