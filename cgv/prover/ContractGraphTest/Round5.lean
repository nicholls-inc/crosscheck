-- Round5.lean
-- Round-5 checker behaviour (docs/design/dataflow-v2.md, "Round 5",
-- "Interface additions"):
-- 1. exact decimal range bounds (`param_min_decimal`/`param_max_decimal`),
--    one scale 10^D per graph, mixed with micros and REAL bounds;
-- 2. call-site suffix paths (`nodes.is_call_site`): checked, errors kept,
--    no warnings;
-- 3. `"guarantee_at"` and `"states_checked"` in the JSON.

import ContractGraph.Types
import ContractGraph.Translation
import ContractGraph.Search
import ContractGraph.StateSearch
import ContractGraph.Main
import ContractGraphTest.Translation
import ContractGraphTest.StateSearch

namespace ContractGraphTest.Round5

open ContractGraph
open ContractGraphTest.Translation (graphOf acceptedGraph)
open ContractGraphTest.Round3 (contains errors warnings firstSuggestion)
open ContractGraphTest.StateSearchTest (sameFindings sameErrorPaths)

/-! ## 1. Exact decimals -/

#guard parseDecimal "0.1234567" == some (1234567, 7)
#guard parseDecimal "-12.50" == some (-1250, 2)
#guard parseDecimal "20000000000000" == some (20000000000000, 0)
#guard parseDecimal "1e5" == none
#guard parseDecimal "1." == none
#guard parseDecimal ".5" == none
#guard parseDecimal "--1" == none
#guard formatScaled 1234567 7 == "0.1234567"
#guard formatScaled (-12500) 4 == "-1.25"
#guard formatScaled 20000000000000 0 == "20000000000000"
-- Inexact rescaling rounds the requested way.
#guard rescale 15 1 0 true == 2
#guard rescale 15 1 0 false == 1
#guard rescale (-15) 1 0 true == -1
#guard rescale (-15) 1 0 false == -2

/-- `make` writes into `R.x`, both `range` rows given by `(min, max)` bound
    rows (built by the caller). -/
def rangeRows (post pre : ContractRow) : Except String ContractGraph :=
  translateRows
    [(1, "make", "function"), (2, "R.x", "model")]
    [{ post with
         nodeId := 1, constraintType := "range", role := some "postcondition"
         sourceFile := "m.py", sourceLine := 3 },
     { pre with
         nodeId := 2, constraintType := "range", role := some "precondition"
         sourceFile := "models.py", sourceLine := 7 }]
    [{ id := 1, sourceId := 1, targetId := 2, relationship := "writes_to" }]

def rangeGraph (post pre : ContractRow)
    (accepted : (rangeRows post pre).isOk := by native_decide) : ContractGraph :=
  acceptedGraph _ accepted

def dec (hi : String) : ContractRow := { nodeId := 0, constraintType := "", maxDecimal := some hi }
def micros (hi : Int) : ContractRow := { nodeId := 0, constraintType := "", maxMicros := some hi }
def real (hi : Float) : ContractRow := { nodeId := 0, constraintType := "", maxReal := some hi }

-- A graph with parameters rejects its rows at each call: `1e-3` is malformed.
/--
error: could not synthesize default value for parameter 'accepted' using tactics
---
error: Tactic `native_decide` evaluated that the proposition
  (rangeRows (dec "1e-3") (dec "1")).isOk = true
is false
-/
#guard_msgs in
example : ContractGraph := rangeGraph (dec "1e-3") (dec "1")

def rangeErrors (g : ContractGraph) : List (String × String) :=
  (errors (runChecker g)).map fun r => (r.sourceGuarantee, r.targetRequirement)

-- 2e13 > 1e13, beyond micros-in-Int64 range and exact.
#guard rangeErrors (rangeGraph (dec "20000000000000") (dec "10000000000000"))
  == [("range ≤ 20000000000000", "range ≤ 10000000000000")]
#guard rangeErrors (rangeGraph (dec "10000000000000") (dec "10000000000000")) == []
-- Seven decimal places, compared exactly.
#guard rangeErrors (rangeGraph (dec "0.1234567") (dec "0.1234567")) == []
#guard rangeErrors (rangeGraph (dec "0.1234568") (dec "0.1234567"))
  == [("range ≤ 0.1234568", "range ≤ 0.1234567")]
#guard contains (firstSuggestion (errors (runChecker (rangeGraph (dec "0.1234568") (dec "0.1234567")))))
  "≤ 0.1234568"
-- The graph's scale is the most decimal places (7), or 6 with micros.
#guard ((rangeGraph (dec "0.1234568") (dec "0.1234567")).nodes.flatMap (·.postconditions)).map
    (fun c => (c.staticBound, c.scale)) == [(some 1234568, 7)]
#guard rangeScale [{ nodeId := 1, constraintType := "range", maxMicros := some 5 }] == 6
#guard rangeScale [{ nodeId := 1, constraintType := "range", maxDecimal := some "1.5" }] == 1
#guard rangeScale [{ nodeId := 1, constraintType := "precision", maxDecimal := some "1.555" }] == 0
-- Mixed: micros-only (0.5) against a 7-place decimal.
#guard rangeErrors (rangeGraph (dec "0.4999999") (micros 500000)) == []
#guard rangeErrors (rangeGraph (dec "0.5000001") (micros 500000))
  == [("range ≤ 0.5000001", "range ≤ 0.5")]
#guard rangeErrors (rangeGraph (micros 500001) (dec "0.5000009"))
  == [("range ≤ 0.500001", "range ≤ 0.5000009")]
-- The decimal is preferred over micros and REAL on the same row.
#guard rangeErrors (rangeGraph { dec "0.4" with maxMicros := some 900000, maxReal := some 0.9 }
    (dec "0.5")) == []
-- A REAL-only guarantee next to decimals: converted at the graph's scale.
#guard rangeErrors (rangeGraph (real 0.7) (dec "0.5000001"))
  == [("range ≤ 0.7", "range ≤ 0.5000001")]
#guard rangeErrors (rangeGraph (real 0.25) (dec "0.2500001")) == []
-- Lower bounds.
#guard rangeErrors (rangeGraph { nodeId := 0, constraintType := "", minDecimal := some "-0.0000001" }
    { nodeId := 0, constraintType := "", minDecimal := some "0" })
  == [("range ≥ -0.0000001", "range ≥ 0")]

/-- `src` (≤ 2.5) → `f` with `max(input_range, 3)` → `R.x` (≤ 2.9999999):
    the dependent literal 3 is scaled with the graph (10^7). -/
def depRangeGraph : ContractGraph :=
  graphOf
    [(1, "src", "function"), (2, "f", "function"), (3, "R.x", "model")]
    [{ nodeId := 1, constraintType := "range", maxDecimal := some "2.5",
       role := some "postcondition", sourceLine := 1 },
     { nodeId := 2, constraintType := "range", dependentExpr := some "max(input_range, 3)",
       role := some "postcondition", sourceLine := 2 },
     { nodeId := 3, constraintType := "range", maxDecimal := some "2.9999999",
       role := some "precondition", sourceLine := 3 }]
    [{ id := 1, sourceId := 1, targetId := 2, relationship := "flows_to" },
     { id := 2, sourceId := 2, targetId := 3, relationship := "writes_to" }]

#guard ((depRangeGraph.nodes[1]!).postconditions.map (·.depExpr))
  == [some (DepExpr.max (.input "input_range") (.lit 30000000))]
#guard (errors (runChecker depRangeGraph)).map (fun r => (r.path, r.sourceGuarantee, r.targetRequirement))
  == [(["src", "f", "R.x"], "range ≤ 3", "range ≤ 2.9999999")]

/-! ## 2. Call-site suffix paths -/

/-- `caller` (3dp) → call site `cs` (`max(input_precision, 2)`) → `M.v` (2dp).
    `cs → M.v` alone is a suffix of `caller → cs → M.v`. -/
def callSiteRows (callSite : Bool) (callerEdge : Bool := true) : Except String ContractGraph :=
  translateRows
    [(1, "caller", "function"),
     { id := 2, name := "cs", kind := "function", isCallSite := callSite },
     (3, "M.v", "model")]
    [{ nodeId := 1, constraintType := "precision", decimalPlaces := some 3,
       role := some "postcondition", sourceLine := 1 },
     { nodeId := 2, constraintType := "precision", dependentExpr := some "max(input_precision, 2)",
       role := some "postcondition", sourceLine := 2 },
     { nodeId := 3, constraintType := "precision", decimalPlaces := some 2,
       role := some "precondition", sourceLine := 3 }]
    ((if callerEdge then [{ id := 1, sourceId := 1, targetId := 2, relationship := "flows_to" }]
      else []) ++
     [{ id := 2, sourceId := 2, targetId := 3, relationship := "writes_to" }])

def callSiteGraph (callSite : Bool) (callerEdge : Bool := true)
    (accepted : (callSiteRows callSite callerEdge).isOk := by native_decide) : ContractGraph :=
  acceptedGraph _ accepted

-- Not a call site: the suffix warns that its bound could not be resolved.
#guard (warnings (runChecker (callSiteGraph false))).map (·.path) == [["cs", "M.v"]]
-- A call site with an incoming edge: no warning; the error stays.
#guard (warnings (runChecker (callSiteGraph true))).isEmpty
#guard (errors (runChecker (callSiteGraph true))).map (fun r => (r.path, r.sourceGuarantee))
  == [(["caller", "cs", "M.v"], "precision ≤ 3")]
#guard (runChecker (callSiteGraph true)).exitCode == 1
-- A call site without an incoming checked edge heads real paths: it warns.
#guard (warnings (runChecker (callSiteGraph true false))).map (·.path) == [["cs", "M.v"]]
-- The path-based reference checker agrees.
#guard [callSiteGraph false, callSiteGraph true, callSiteGraph true false].all sameFindings
#guard [callSiteGraph false, callSiteGraph true, callSiteGraph true false].all sameErrorPaths
#guard (suffixHeadSet (callSiteGraph true)).toList == [2]
#guard (suffixHeadSet (callSiteGraph true false)).toList == []

/-- The call site's own static bound (4dp) into `M.v` (2dp): an error on the
    suffix path; it is kept (from the call site, the shortest path). -/
def callSiteErrorGraph : ContractGraph :=
  graphOf
    [(1, "caller", "function"), { id := 2, name := "cs", kind := "function", isCallSite := true },
     (3, "M.v", "model")]
    [{ nodeId := 2, constraintType := "precision", decimalPlaces := some 4,
       role := some "postcondition", sourceLine := 2 },
     { nodeId := 3, constraintType := "precision", decimalPlaces := some 2,
       role := some "precondition", sourceLine := 3 },
     { nodeId := 3, constraintType := "nullability", nullable := some 0,
       role := some "precondition", sourceLine := 3 }]
    [{ id := 1, sourceId := 1, targetId := 2, relationship := "flows_to" },
     { id := 2, sourceId := 2, targetId := 3, relationship := "writes_to" }]

#guard (errors (runChecker callSiteErrorGraph)).map (·.path) == [["cs", "M.v"]]
-- The missing-nullability warning on the hop cs → M.v is reported, because
-- the longer path caller → cs → M.v reaches that hop too (with the same
-- composed state). The state-based checker's witness is the shortest walk to
-- the state (from cs); the path-based checker names the longer path, the
-- only unsuppressed one.
#guard (warnings (runChecker callSiteErrorGraph)).map (fun r => (r.path, r.hop))
  == [(["cs", "M.v"], ["cs", "M.v"])]
#guard (warnings (runCheckerPaths callSiteErrorGraph)).map (·.path) == [["caller", "cs", "M.v"]]
#guard sameFindings callSiteErrorGraph

/-! ## 3. JSON -/

def json (g : ContractGraph) : String := outputToJson (runChecker g)

-- An error's guarantee_at is its source constraint's own location.
#guard contains (json (rangeGraph (dec "0.1234568") (dec "0.1234567")))
  "\"guarantee_at\": {\"file\": \"m.py\", \"line\": 3}"
#guard ((errors (runChecker (rangeGraph (dec "0.1234568") (dec "0.1234567")))).map
    (fun r => ((guaranteeAt r).file, (guaranteeAt r).line))) == [("m.py", 3)]
-- A warning's is the target requirement's location.
#guard (warnings (runChecker (callSiteGraph false))).map (fun r => (guaranteeAt r).line) == [3]
-- states_checked (and paths_checked, the same for the state-based checker).
#guard contains (json (callSiteGraph true)) "\"paths_checked\": 3, \"states_checked\": 3"
#guard (runCheckerPaths (callSiteGraph true)).summary.statesChecked == 0

end ContractGraphTest.Round5
