-- Round3.lean
-- Round-3 checker behaviour (docs/design/dataflow-v2.md, "Round 3"):
-- 1. pruned, indexed, prefix-sharing search: same paths and results as the
--    naive enumerator; path budget (exit code 2, "incomplete");
-- 2. node locations in results (path head for errors, hop source for
--    warnings);
-- 3. write/call sites, and deduplication by site;
-- 4. exact range bounds in micros;
-- 5. JSON choices lists, and choices in the warn-when-missing kinds;
-- 6. warnings only where a requirement can reject the value.

import ContractGraph.Types
import ContractGraph.Checker
import ContractGraph.Composition
import ContractGraph.Diagnostics
import ContractGraph.Translation
import ContractGraph.Search
import ContractGraph.Main
import ContractGraphTest.DataflowV2
import ContractGraphTest.DedupeTest
import ContractGraphTest.NoErrorsSoundness
import ContractGraphTest.TransitiveDemo
import ContractGraphTest.BugReport1
import ContractGraphTest.NullableDemo

namespace ContractGraphTest.Round3

open ContractGraph

def contains (s sub : String) : Bool := (s.splitOn sub).length > 1
def errors (o : CheckOutput) : List ResultEntry := o.results.filter (·.severity == "error")
def warnings (o : CheckOutput) : List ResultEntry := o.results.filter (·.severity == "warning")
def firstSuggestion (rs : List ResultEntry) : String := (rs.head?.map (·.suggestion)).getD ""

/-! ## 1. Pruned search = naive enumeration -/

def names (p : List Edge) : List String := p.map (·.source.name) ++ [(p.getLast?.map (·.target.name)).getD ""]

/-- A key identifying a result entry independently of report order. -/
def key (r : ResultEntry) : String :=
  s!"{r.severity}|{r.path}|{r.hop}|{r.sourceGuarantee}|{r.targetRequirement}|{r.target.line}|{r.source.line}"

/-- The checker's output on `g` built the pre-round-3 way: the naive
    enumerator (one search per (function, model) pair, no pruning, no
    index), `checkPath` from scratch per path, then deduplication. -/
def naiveResults (g : ContractGraph) : List ResultEntry :=
  dedupeResults (collectResults ((enumeratePathsNaive g).map fun p => (p, checkPath p)))

def sameMultiset (xs ys : List String) : Bool :=
  xs.length == ys.length && xs.all (fun x => xs.count x == ys.count x)

/-- Same paths, same per-path results, same report as the naive way. -/
def agreesWithNaive (g : ContractGraph) : Bool :=
  sameMultiset ((enumeratePaths g).map (toString ∘ names))
      ((enumeratePathsNaive g).map (toString ∘ names)) &&
  (checkAllPaths g).all (fun (p, rs) => rs.length == (checkPath p).length) &&
  sameMultiset ((runChecker g).results.map key) ((naiveResults g).map key)

#guard agreesWithNaive DataflowV2.lowerGraph
#guard agreesWithNaive DataflowV2.overrideGraph
#guard agreesWithNaive DataflowV2.emptyOverrideGraph
#guard agreesWithNaive DataflowV2.noOverrideGraph
#guard agreesWithNaive DataflowV2.paramGraph
#guard agreesWithNaive DataflowV2.depGraph
#guard agreesWithNaive DataflowV2.depGraphClean
#guard agreesWithNaive DataflowV2.callsGraph
#guard agreesWithNaive DataflowV2.callsOnlyGraph
#guard agreesWithNaive DataflowV2.finalHopGraph
#guard agreesWithNaive DataflowV2.twoHopGraph
#guard agreesWithNaive DedupeTest.graph
#guard agreesWithNaive NoErrorsSoundness.graph
#guard agreesWithNaive NoErrorsSoundness.chainGraph
#guard agreesWithNaive NoErrorsSoundness.cycleGraph
#guard agreesWithNaive TransitiveDemo.transitiveGraph
#guard agreesWithNaive BugReport1.bug1Graph
#guard agreesWithNaive NullableDemo.nullableGraph

/-- Pruning at work: `f` also flows into `dead` (reaches no model) and `g2`
    reaches two models. -/
def fnode (i : Nat) (n : String) (post : List Constraint := []) : Node :=
  { id := i, name := n, kind := "function", preconditions := [], postconditions := post }
def mnode (i : Nat) (n : String) (pre : List Constraint) : Node :=
  { id := i, name := n, kind := "model", preconditions := pre, postconditions := [] }
def c (k : ConstraintKind) (b : Int) (line : Nat := 1) : Constraint :=
  { kind := k, staticBound := some b, sourceFile := "t.py", sourceLine := line,
    verificationLevel := .extracted }
def flow (s t : Node) : Edge := { source := s, target := t, relationship := .flowsTo }
def write (s t : Node) : Edge := { source := s, target := t, relationship := .writesTo }

def fN := fnode 1 "f" [c .precision 4]
def deadN := fnode 2 "dead" [c .precision 9]
def g2N := fnode 3 "g2" [c .precision 3]
def aN := mnode 10 "M.a" [c .precision 2]
def bN := mnode 11 "M.b" [c .precision 3]

def prunedGraph : ContractGraph :=
  { nodes := [fN, deadN, g2N, aN, bN]
    edges := [flow fN deadN, flow deadN deadN, flow fN g2N, write g2N aN, write g2N bN,
              write fN bN] }

#guard agreesWithNaive prunedGraph
-- The index never lists the edge into `dead` (it reaches no model) ...
#guard ((searchSetup prunedGraph).out 1).map (·.target.name) == ["g2", "M.b"]
-- ... while the naive search explores it.
#guard (enumeratePaths prunedGraph).length == 5
#guard (runCheckerPaths prunedGraph).summary.pathsChecked == 5

/-! ### Path budget (path-based reference checker `runCheckerPaths`) -/

-- chainGraph has 3 data paths.
#guard (runCheckerPaths NoErrorsSoundness.chainGraph 3).exitCode == 0
#guard (runCheckerPaths NoErrorsSoundness.chainGraph 3).summary.pathsChecked == 3
-- A budget of 2 is exceeded: exit code 2, one "incomplete" error, no paths checked.
#guard (runCheckerPaths NoErrorsSoundness.chainGraph 2).exitCode == 2
#guard (runCheckerPaths NoErrorsSoundness.chainGraph 2).results.map (fun r => (r.status, r.severity))
  == [("incomplete", "error")]
#guard contains (firstSuggestion (runCheckerPaths NoErrorsSoundness.chainGraph 2).results)
  "--max-paths 2"
#guard (runCheckerPaths NoErrorsSoundness.chainGraph 2).summary.pathsChecked == 0
#guard (runCheckerPaths NoErrorsSoundness.chainGraph 2).summary.edgesChecked == 3
#guard contains (outputToJson (runCheckerPaths NoErrorsSoundness.chainGraph 2)) "\"exit_code\": 2"
#guard contains (outputToJson (runCheckerPaths NoErrorsSoundness.chainGraph 2)) "\"status\": \"incomplete\""
-- Command-line options (state-based checker; `--max-paths` is an alias of
-- `--max-states`).
#guard (parseOptions []).toOption == some {}
#guard (parseOptions ["--max-paths", "17"]).toOption == some { maxStates := 17 }
#guard (parseOptions ["--max-states", "17"]).toOption == some { maxStates := 17 }
#guard (parseOptions ["--max-states-per-edge", "5", "--max-states", "9"]).toOption
  == some { maxStates := 9, maxPerEdge := 5 }
#guard (parseOptions ["--max-paths", "x"]).toOption == none
#guard (parseOptions ["--max-states"]).toOption == none
#guard (parseOptions ["--bogus"]).toOption == none

/-- A diamond: `a_i → a_{i+1}` and `a_i → b_{i+1}`, `b_i → a_{i+1}`, `b_i →
    b_{i+1}`, with `a_n` writing to a 2dp field; `src` (3dp) feeds `a1`;
    each `a_i`/`b_i` has `max(input_precision, 2)`. 2^n paths from src. -/
def dep : Constraint :=
  { kind := .precision, depExpr := some (.max (.input "input_precision") (.lit 2)),
    sourceFile := "d.py", sourceLine := 1, verificationLevel := .extracted }
def diamond (n : Nat) : ContractGraph :=
  let a (i : Nat) := fnode (2 * i) s!"a{i}" [dep]
  let b (i : Nat) := fnode (2 * i + 1) s!"b{i}" [dep]
  let src := fnode 1000 "src" [c .precision 3]
  let out := mnode 2000 "Out.v" [c .precision 2]
  let layers := (List.range n).map (· + 1)
  { nodes := [src, out] ++ layers.flatMap (fun i => [a i, b i])
    edges := [flow src (a 1), write (a n) out] ++
      (List.range (n - 1)).flatMap (fun j =>
        let i := j + 1
        [flow (a i) (a (i+1)), flow (a i) (b (i+1)), flow (b i) (a (i+1)), flow (b i) (b (i+1))]) }

-- 2^6 paths from src (a1 and a8 fixed), 2 · 2^(7-i) from layer i, and a8...
#guard (runCheckerPaths (diamond 8)).summary.pathsChecked == 319
#guard agreesWithNaive (diamond 6)
#guard (errors (runChecker (diamond 8))).map (·.sourceGuarantee) == ["precision ≤ 3"]
#guard (runCheckerPaths (diamond 8) 100).exitCode == 2

-- Soundness still holds for the budgeted runner: exit code 0 with any
-- budget gives stepwise soundness of every data path.
example (g : ContractGraph) (m : Nat) (h : (runCheckerPaths g m).exitCode = 0) :
    ∀ p, IsDataPath g p → stepwiseSound p := runCheckerPaths_sound_all g h
-- ... and for the state-based one, with any state budgets.
example (g : ContractGraph) (m k : Nat) (h : (runChecker g m k).exitCode = 0) :
    ∀ p, IsDataPath g p → stepwiseSound p := runChecker_sound_all g h

/-! ## 2. Node locations (M1, M2) and 3. sites (M3) -/

/-- five() (line 9) → keep(p) (line 14) → S.e (3dp); keep has a dependent
    postcondition (line 15); the write is at w.py:30. -/
def locGraph : ContractGraph :=
  buildGraph
    [{ id := 1, name := "five", kind := "function", sourceFile := "m.py", sourceLine := 9 },
     { id := 2, name := "keep", kind := "function", sourceFile := "m.py", sourceLine := 14 },
     { id := 3, name := "S.e", kind := "model", sourceFile := "models.py", sourceLine := 3 }]
    [{ nodeId := 1, constraintType := "precision", decimalPlaces := some 5,
       role := some "postcondition", sourceFile := "m.py", sourceLine := 10 },
     { nodeId := 2, constraintType := "precision", dependentExpr := some "max(input_precision, 2)",
       role := some "postcondition", sourceFile := "m.py", sourceLine := 15 },
     { nodeId := 3, constraintType := "precision", decimalPlaces := some 3,
       role := some "precondition", sourceFile := "models.py", sourceLine := 4 }]
    [{ id := 1, sourceId := 1, targetId := 2, relationship := .flowsTo,
       siteFile := "w.py", siteLine := 29 },
     { id := 2, sourceId := 2, targetId := 3, relationship := .writesTo,
       siteFile := "w.py", siteLine := 30 }]

#guard locGraph.nodes.map (fun n => (n.sourceFile, n.sourceLine))
  == [("m.py", 9), ("m.py", 14), ("models.py", 3)]
#guard locGraph.edges.map (fun e => (e.siteFile, e.siteLine)) == [("w.py", 29), ("w.py", 30)]
-- The composed error is reported at the path head's definition (line 9), not
-- at keep's line; the failing hop keep → S.e is at site w.py:30.
#guard (errors (runChecker locGraph)).map
    (fun r => (r.source.file, r.source.line, r.source.name, r.hop, r.site.file, r.site.line))
  == [("m.py", 9, "five", ["keep", "S.e"], "w.py", 30)]
-- The unresolved-bound warning on keep → S.e is at keep's definition (the hop
-- source), and shows S.e's requirement.
#guard (warnings (runChecker locGraph)).map
    (fun r => (r.source.file, r.source.line, r.source.name, r.targetRequirement, r.site.line))
  == [("m.py", 14, "keep", "precision ≤ 3", 30)]
#guard contains (outputToJson (runChecker locGraph)) "\"site\": {\"file\": \"w.py\", \"line\": 30}"

/-- third() (4dp) written into a 2dp field at three different sites. -/
def siteGraph : ContractGraph :=
  buildGraph
    [{ id := 1, name := "third", kind := "function", sourceFile := "t.py", sourceLine := 1 },
     { id := 2, name := "P.amount", kind := "model", sourceFile := "models.py", sourceLine := 5 }]
    [{ nodeId := 1, constraintType := "precision", decimalPlaces := some 4,
       role := some "postcondition", sourceFile := "t.py", sourceLine := 2 },
     { nodeId := 2, constraintType := "precision", decimalPlaces := some 2,
       role := some "precondition", sourceFile := "models.py", sourceLine := 5 }]
    [{ id := 1, sourceId := 1, targetId := 2, relationship := .writesTo, siteFile := "a.py", siteLine := 10 },
     { id := 2, sourceId := 1, targetId := 2, relationship := .writesTo, siteFile := "b.py", siteLine := 20 },
     { id := 3, sourceId := 1, targetId := 2, relationship := .writesTo, siteFile := "c.py", siteLine := 30 }]

-- Findings at different sites are not merged.
#guard (errors (runChecker siteGraph)).map (fun r => (r.site.file, r.site.line))
  == [("a.py", 10), ("b.py", 20), ("c.py", 30)]
-- Without sites (old databases) they are one finding.
def siteGraphNoSites : ContractGraph :=
  { siteGraph with edges := siteGraph.edges.map fun e => { e with siteFile := "", siteLine := 0 } }
#guard (errors (runChecker siteGraphNoSites)).length == 1

/-! ## 4. Exact bounds in micros (D4) -/

#guard formatMicros 500000 == "0.5"
#guard formatMicros (-1000000) == "-1"
#guard formatMicros 100000000 == "100"
#guard formatMicros (-500000) == "-0.5"
#guard formatMicros 1 == "0.000001"
#guard formatMicros 1250000 == "1.25"
#guard formatMicros 0 == "0"

-- Legacy REAL bounds: exact values stay exact; others round conservatively.
#guard legacyMicros 0.5 true true == 500000
#guard legacyMicros 0.3 true false == 300000
#guard legacyMicros (-1.0) false true == -1000000
-- 0.1234567: a requirement's upper bound rounds down, a guarantee's up;
-- a requirement's lower bound rounds up, a guarantee's down.
#guard legacyMicros 0.1234567 true true == 123456
#guard legacyMicros 0.1234567 true false == 123457
#guard legacyMicros 0.1234567 false true == 123457
#guard legacyMicros 0.1234567 false false == 123456

/-- ratio: Field(ge=0.0, le=0.5); writes 0.7. -/
def microsGraph (written : Int) : ContractGraph :=
  buildGraph
    [(1, "make", "function"), (2, "R.ratio", "model")]
    [{ nodeId := 1, constraintType := "range", minMicros := some written, maxMicros := some written,
       role := some "postcondition", sourceLine := 1 },
     { nodeId := 2, constraintType := "range", minMicros := some 0, maxMicros := some 500000,
       role := some "precondition", sourceLine := 2 }]
    [{ id := 1, sourceId := 1, targetId := 2, relationship := .writesTo }]

#guard (errors (runChecker (microsGraph 700000))).map (fun r => (r.sourceGuarantee, r.targetRequirement))
  == [("range ≤ 0.7", "range ≤ 0.5")]
#guard contains (firstSuggestion (errors (runChecker (microsGraph 700000)))) "≤ 0.7"
#guard (errors (runChecker (microsGraph 500000))).isEmpty
#guard (errors (runChecker (microsGraph (-1000000)))).map (fun r => (r.sourceGuarantee, r.targetRequirement))
  == [("range ≥ -1", "range ≥ 0")]

-- A range dependent expression's literals are scaled to micros.
def depRow (kind e : String) : ContractRow :=
  { nodeId := 1, constraintType := kind, dependentExpr := some e }
#guard (translateContractRow (depRow "range" "max(input_range, 3)")).map (·.depExpr)
  == [some (DepExpr.max (.input "input_range") (.lit 3000000))]
-- Other kinds are not scaled.
#guard (translateContractRow (depRow "precision" "max(input_precision, 3)")).map (·.depExpr)
  == [some (DepExpr.max (.input "input_precision") (.lit 3))]

/-! ## 5. Choices (D5) -/

#guard parseChoices "[\"a\", \"x\"]" == some ["a", "x"]
#guard parseChoices " [ \"a,b\" , \"q\\\"\" ] " == some ["a,b", "q\""]
#guard parseChoices "[]" == some []
#guard parseChoices "[\"\\u0041\"]" == some ["A"]
#guard parseChoices "a,x" == some ["a", "x"]
#guard parseChoices "[\"a\"" == none
#guard parseChoices "[\"a\",]" == none

-- A malformed value is rejected when the database is read, not translated
-- as "no constraint" (`readContractGraph` fails, exit code 2).
#guard (({ nodeId := 1, constraintType := "choices", choices := some "[\"a\"" } : ContractRow).malformed).isSome
#guard (({ nodeId := 1, constraintType := "choices", choices := some "[\"a\"]" } : ContractRow).malformed).isNone
#guard (({ nodeId := 1, constraintType := "choices", choices := some "a,b" } : ContractRow).malformed).isNone
#guard (({ nodeId := 1, constraintType := "range", maxDecimal := some "1e-3" } : ContractRow).malformed).isSome
#guard (({ nodeId := 1, constraintType := "range", minDecimal := some "x" } : ContractRow).malformed).isSome
#guard (({ nodeId := 1, constraintType := "range", minDecimal := some "-0.25", maxDecimal := some "10" } : ContractRow).malformed).isNone

/-- status: CharField(choices=[("a", ...), ("x", ...)]), JSON-encoded. -/
def choicesGraph (written : Option String) : ContractGraph :=
  buildGraph
    [(1, "make", "function"), (2, "P.status", "model")]
    ((match written with
      | some w => [{ nodeId := 1, constraintType := "choices", choices := some w,
                     role := some "postcondition", sourceLine := 1 }]
      | none => []) ++
     [{ nodeId := 1, constraintType := "length", maxLength := some 2,
        role := some "postcondition", sourceLine := 1 },
      { nodeId := 2, constraintType := "choices", choices := some "[\"a\", \"x\"]",
        role := some "precondition", sourceLine := 7 }])
    [{ id := 1, sourceId := 1, targetId := 2, relationship := .writesTo }]

#guard (errors (runChecker (choicesGraph (some "[\"zz\"]")))).map (·.sourceGuarantee)
  == ["choices in [zz]"]
#guard (errors (runChecker (choicesGraph (some "[\"a\"]")))).isEmpty
#guard (warnings (runChecker (choicesGraph (some "[\"a\"]")))).isEmpty
-- No choices fact on the write: a warning (the requirement passes vacuously).
#guard (warnings (runChecker (choicesGraph none))).map (fun r => (r.sourceGuarantee, r.targetRequirement))
  == [("choices (unspecified)", "choices in [a, x]")]

/-! ## 6. Warnings only where they matter (N1, N2, M4) -/

/-- round2(p) has `max(input_precision, 2)`; it flows into a str field with
    a length requirement only, and into a nullable DateTimeField. -/
def noiseGraph : ContractGraph :=
  buildGraph
    [(1, "round2", "function"), (2, "P.label", "model"), (3, "P.when", "model"),
     (4, "P.amount", "model")]
    [{ nodeId := 1, constraintType := "precision", dependentExpr := some "max(input_precision, 2)",
       role := some "postcondition", sourceLine := 1 },
     { nodeId := 1, constraintType := "length", maxLength := some 10,
       role := some "postcondition", sourceLine := 1 },
     { nodeId := 2, constraintType := "length", maxLength := some 20,
       role := some "precondition", sourceLine := 2 },
     { nodeId := 3, constraintType := "nullability", nullable := some 1,
       role := some "precondition", sourceLine := 3 },
     { nodeId := 4, constraintType := "nullability", nullable := some 0,
       role := some "precondition", sourceLine := 4 },
     { nodeId := 4, constraintType := "precision", decimalPlaces := some 2,
       role := some "precondition", sourceLine := 5 }]
    [{ id := 1, sourceId := 1, targetId := 2, relationship := .writesTo },
     { id := 2, sourceId := 1, targetId := 3, relationship := .writesTo },
     { id := 3, sourceId := 1, targetId := 4, relationship := .writesTo }]

-- No unresolved-precision warning into P.label (no precision requirement)
-- or P.when; no missing-nullability warning into the nullable P.when.
#guard (warnings (runChecker noiseGraph)).all (·.hop == ["round2", "P.amount"])
-- Into P.amount: the unresolved bound shows the precision requirement it
-- would be checked against (not "(dependent)" twice); nullability warns
-- because P.amount is non-null.
#guard (warnings (runChecker noiseGraph)).map (fun r => (r.sourceGuarantee, r.targetRequirement))
  == [("precision (dependent)", "precision ≤ 2"), ("nullability (unspecified)", "non-null")]
#guard (runChecker noiseGraph).exitCode == 0

-- The hop helpers directly.
#guard (collectUnresolvedWarnings (noiseGraph.edges[0]!).source (noiseGraph.edges[0]!).target).isEmpty
#guard (collectMissingPostconditionWarnings (noiseGraph.edges[1]!).source
    (noiseGraph.edges[1]!).target).isEmpty
#guard (collectMissingPostconditionWarnings (noiseGraph.edges[2]!).source
    (noiseGraph.edges[2]!).target).length == 1

end ContractGraphTest.Round3
