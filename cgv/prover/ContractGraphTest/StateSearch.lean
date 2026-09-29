-- StateSearch.lean (tests)
-- The state-based checker (`runChecker`, StateSearch.lean):
-- 1. the same findings as the path-based reference checker (`runCheckerPaths`)
--    on every test graph, and the same error paths;
-- 2. cycles: terminating when composition settles, the per-edge state cap
--    (exit code 2, naming the cycle) when a bound keeps growing, and the
--    total state budget;
-- 3. parallel per-parameter edges: one state per edge, however many paths;
-- 4. the diamond at a size the path enumeration cannot reach.

import ContractGraph.Types
import ContractGraph.Composition
import ContractGraph.Translation
import ContractGraph.Search
import ContractGraph.StateSearch
import ContractGraph.Main
import ContractGraphTest.Round3

namespace ContractGraphTest.StateSearchTest

open ContractGraph
open ContractGraphTest.Round3 (fnode mnode c flow write diamond prunedGraph locGraph siteGraph
  siteGraphNoSites microsGraph choicesGraph noiseGraph contains errors warnings firstSuggestion)

/-! ## 1. Same findings as the path-based checker -/

/-- A finding without its witness path (path, path head, verification level). -/
def findingOf (r : ResultEntry) : String :=
  s!"{r.status}|{r.severity}|{r.hop}|{r.site.file}:{r.site.line}|{r.sourceGuarantee}|" ++
  s!"{r.targetRequirement}|{r.target.name}|{r.target.file}:{r.target.line}|{r.suggestion}"

/-- An error with its witness path and path head. -/
def errorWithPath (r : ResultEntry) : String :=
  s!"{findingOf r}|{r.path}|{r.source.name}|{r.source.line}"

def sameMultiset (xs ys : List String) : Bool :=
  xs.length == ys.length && xs.all (fun x => xs.count x == ys.count x)

/-- The state-based and the path-based checker report the same findings
    (modulo witness path) and the same exit code. -/
def sameFindings (g : ContractGraph) : Bool :=
  sameMultiset ((runChecker g).results.map findingOf) ((runCheckerPaths g).results.map findingOf) &&
  (runChecker g).exitCode == (runCheckerPaths g).exitCode

/-- ... and, here, the same witness paths for errors (the shortest). -/
def sameErrorPaths (g : ContractGraph) : Bool :=
  sameMultiset ((errors (runChecker g)).map errorWithPath)
    ((errors (runCheckerPaths g)).map errorWithPath)

def testGraphs : List ContractGraph :=
  [DataflowV2.lowerGraph, DataflowV2.overrideGraph, DataflowV2.emptyOverrideGraph,
   DataflowV2.noOverrideGraph, DataflowV2.paramGraph, DataflowV2.depGraph,
   DataflowV2.depGraphClean, DataflowV2.callsGraph, DataflowV2.callsOnlyGraph,
   DataflowV2.finalHopGraph, DataflowV2.twoHopGraph, DedupeTest.graph,
   NoErrorsSoundness.graph, NoErrorsSoundness.chainGraph, NoErrorsSoundness.cycleGraph,
   TransitiveDemo.transitiveGraph, BugReport1.bug1Graph, NullableDemo.nullableGraph,
   prunedGraph, diamond 6, diamond 8, locGraph, siteGraph, siteGraphNoSites,
   microsGraph 700000, microsGraph 500000, microsGraph (-1000000),
   choicesGraph (some "[\"zz\"]"), choicesGraph (some "[\"a\"]"), choicesGraph none, noiseGraph]

#guard testGraphs.all sameFindings
#guard testGraphs.all sameErrorPaths
-- Not vacuous: the test graphs have findings of both severities.
#guard (testGraphs.map (fun g => (errors (runChecker g)).length)).sum ≥ 20
#guard (testGraphs.map (fun g => (warnings (runChecker g)).length)).sum ≥ 10

-- Each composed hop is checked once: 53 states on diamond 8 (30 edges) against
-- 319 paths.
#guard (runCheckerPaths (diamond 8)).summary.pathsChecked == 319
#guard (runChecker (diamond 8)).summary.pathsChecked == 53

/-! ## 2. Cycles -/

def dep (e : DepExpr) (line : Nat := 1) : Constraint :=
  { kind := .precision, depExpr := some e, sourceFile := "d.py", sourceLine := line,
    verificationLevel := .extracted }

/-- `src` (3dp) → `p` → `q` → `p` (a cycle) and `q` → `M.v` (2dp); `p` and
    `q` both have the dependent postcondition `post`. -/
def cycleWith (post : Constraint) : ContractGraph :=
  let src := fnode 1 "src" [c .precision 3]
  let p := fnode 2 "p" [post]
  let q := fnode 3 "q" [post]
  let m := mnode 4 "M.v" [c .precision 2]
  { nodes := [src, p, q, m]
    edges := [flow src p, flow p q, flow q p, write q m] }

-- `max(input_precision, 2)` settles after one round: the exploration ends,
-- with the same findings as the simple paths.
def settling := cycleWith (dep (.max (.input "input_precision") (.lit 2)))
#guard (runChecker settling).exitCode == 1
#guard sameFindings settling
#guard sameErrorPaths settling
#guard (errors (runChecker settling)).map (fun r => (r.path, r.sourceGuarantee))
  == [(["src", "p", "q", "M.v"], "precision ≤ 3")]
-- A static cycle (NoErrorsSoundness.cycleGraph: a ⇄ b) ends too.
#guard (runChecker NoErrorsSoundness.cycleGraph).exitCode == 0

-- `add(input_precision, 1)` grows by one per hop around the cycle: the
-- per-edge cap is exceeded, the run is incomplete (exit code 2) and the
-- message names the hop and the cycle.
def growing := cycleWith (dep (.add (.input "input_precision") (.lit 1)))
#guard (runChecker growing).exitCode == 2
#guard (runChecker growing).results.map (fun r => (r.status, r.severity)) == [("incomplete", "error")]
#guard contains (firstSuggestion (runChecker growing).results) "--max-states-per-edge 64"
#guard contains (firstSuggestion (runChecker growing).results) "Cycle: "
#guard contains (firstSuggestion (runChecker growing).results) "p -> q -> p"
#guard (runChecker growing).summary.pathsChecked == 0
-- A larger cap only takes longer; the growth never stops.
#guard (runChecker growing defaultMaxStates 500).exitCode == 2
-- The path-based checker sees only the simple paths and finishes (with an
-- error: 3 + 1 + 1 = 5 > 2).
#guard (runCheckerPaths growing).exitCode == 1

-- The total state budget (`--max-states`, alias `--max-paths`).
#guard (runChecker NoErrorsSoundness.chainGraph 3).exitCode == 0
#guard (runChecker NoErrorsSoundness.chainGraph 3).summary.pathsChecked == 3
#guard (runChecker NoErrorsSoundness.chainGraph 2).exitCode == 2
#guard contains (firstSuggestion (runChecker NoErrorsSoundness.chainGraph 2).results)
  "--max-states 2"
#guard (runChecker NoErrorsSoundness.chainGraph 2).summary.edgesChecked == 3
#guard contains (outputToJson (runChecker NoErrorsSoundness.chainGraph 2)) "\"status\": \"incomplete\""

-- Soundness holds whatever the budgets.
example (g : ContractGraph) (m k : Nat) (h : (runChecker g m k).exitCode = 0) :
    ∀ p, IsDataPath g p → stepwiseSound p := runChecker_sound_all g h

/-! ## 3. Parallel per-parameter edges -/

/-- `h_i(a, b, c)` for `i = 1..n`: `h_{i-1}` is passed as all three
    arguments of `h_i` (three `flows_to` edges, one per parameter, each
    target copy with that parameter's precondition), `h_n` writes a 2dp
    field. Every `h_i` has 3dp; parameter `c` requires ≤ 2dp. 3^n paths
    from `h_0`. -/
def perParam (n : Nat) : ContractGraph :=
  let pre (subj : String) (b : Int) : Constraint :=
    { c .precision b with subject := some subj }
  let h (i : Nat) : Node :=
    { fnode i s!"h{i}" [c .precision 3] with
      preconditions := [pre "a" 6, pre "b" 6, pre "c" 2] }
  let copy (i : Nat) (subj : String) : Node :=
    { h i with preconditions := (h i).preconditions.filter (·.subject == some subj) }
  let m := mnode 1000 "M.v" [c .precision 3]
  let layers := List.range n
  { nodes := layers.map h ++ [h n, m]
    edges := layers.flatMap (fun i => ["a", "b", "c"].map fun subj =>
        { source := h i, target := copy (i + 1) subj, relationship := .flowsTo }) ++
      [write (h n) m] }

#guard (enumeratePaths (perParam 4)).length == 81 + 27 + 9 + 3 + 1
-- ... one state per edge: the hops into `h_i` over different parameters
-- leave the same composed state (`normHop` drops the target copy's
-- preconditions when it becomes the next hop's source).
#guard (runChecker (perParam 4)).summary.pathsChecked == (perParam 4).edges.length
#guard sameFindings (perParam 4)
#guard sameErrorPaths (perParam 4)
-- One error per layer (the `c` parameter), reported once each.
#guard (errors (runChecker (perParam 4))).map (·.hop)
  == (List.range 4).map (fun i => [s!"h{i}", s!"h{i + 1}"])
-- At 30 layers (3^29 paths from h0) it is still one state per edge.
#guard (runChecker (perParam 30)).summary.pathsChecked == 91
#guard (errors (runChecker (perParam 30))).length == 30

/-! ## 4. Diamond at scale -/

-- 2^38 paths from src alone; a few hundred states.
#guard (runChecker (diamond 40)).exitCode == 1
#guard (errors (runChecker (diamond 40))).map (·.sourceGuarantee) == ["precision ≤ 3"]
#guard (runChecker (diamond 40)).summary.pathsChecked < 400

end ContractGraphTest.StateSearchTest
