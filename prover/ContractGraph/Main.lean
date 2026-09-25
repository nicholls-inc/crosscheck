-- Main.lean

import ContractGraph.Types
import ContractGraph.BehaviorModel
import ContractGraph.DependentExpr
import ContractGraph.Translation
import ContractGraph.Checker
import ContractGraph.Composition
import ContractGraph.Diagnostics
import ContractGraph.Search

namespace ContractGraph

/-- Escape a string for JSON output. -/
private def jsonEscape (s : String) : String :=
  s.replace "\\" "\\\\" |>.replace "\"" "\\\"" |>.replace "\n" "\\n"

/-- Format a SourceLocation as JSON. -/
private def sourceLocationToJson (loc : SourceLocation) : String :=
  s!"\{\"file\": \"{jsonEscape loc.file}\", \"line\": {loc.line}, \"name\": \"{jsonEscape loc.name}\"}"

/-- Format a SiteLocation as JSON. -/
private def siteLocationToJson (loc : SiteLocation) : String :=
  s!"\{\"file\": \"{jsonEscape loc.file}\", \"line\": {loc.line}}"

/-- Format a list of strings as JSON array. -/
private def stringListToJson (xs : List String) : String :=
  let items := xs.map (fun s => s!"\"{jsonEscape s}\"")
  s!"[{", ".intercalate items}]"

/-- Format a ResultEntry as JSON. -/
private def resultEntryToJson (r : ResultEntry) : String :=
  s!"\{\"status\": \"{r.status}\", " ++
  s!"\"severity\": \"{r.severity}\", " ++
  s!"\"source\": {sourceLocationToJson r.source}, " ++
  s!"\"target\": {sourceLocationToJson r.target}, " ++
  s!"\"path\": {stringListToJson r.path}, " ++
  s!"\"hop\": {stringListToJson r.hop}, " ++
  s!"\"site\": {siteLocationToJson r.site}, " ++
  s!"\"source_guarantee\": \"{jsonEscape r.sourceGuarantee}\", " ++
  s!"\"target_requirement\": \"{jsonEscape r.targetRequirement}\", " ++
  s!"\"verification_level\": \"{r.verificationLevel}\", " ++
  s!"\"suggestion\": \"{jsonEscape r.suggestion}\"}"

/-- Format CheckOutput as JSON. -/
def outputToJson (output : CheckOutput) : String :=
  let resultsJson := output.results.map resultEntryToJson
  s!"\{\"summary\": \{" ++
  s!"\"contracts_checked\": {output.summary.contractsChecked}, " ++
  s!"\"edges_checked\": {output.summary.edgesChecked}, " ++
  s!"\"paths_checked\": {output.summary.pathsChecked}}, " ++
  s!"\"results\": [{", ".intercalate resultsJson}], " ++
  s!"\"exit_code\": {output.exitCode}}"

/-- Node names along a path: every edge's source, then the last target. -/
def pathNamesOf (path : List Edge) : List String :=
  match path with
  | [] => []
  | edges => (edges.map (·.source.name)) ++
    match edges.getLast? with
    | some last => [last.target.name]
    | none => []

/-- Whether a result is consistent. -/
def CheckResult.isConsistent : CheckResult → Bool
  | .consistent => true
  | .inconsistent _ => false

/-- The result entries of one checked path: one per inconsistent result. -/
def collectEntries (path : List Edge) (results : List CheckResult) : List ResultEntry :=
  if results.all (·.isConsistent) then [] else
  let pathNames := pathNamesOf path
  let verLevel := pathVerificationLevel path
  let head := path.head?.map (·.source)
  results.filterMap fun r =>
    match r with
    | .consistent => none
    | .inconsistent diag =>
      some (buildResultEntry { diag with path := pathNames } pathNames verLevel head)

/-- Collect all inconsistencies from path check results. -/
def collectResults (pathResults : List (List Edge × List CheckResult))
    : List ResultEntry :=
  pathResults.flatMap fun (path, results) => collectEntries path results

/-- Two entries report the same inconsistency when everything except the
    path (and so the path head) matches: the same source constraint (by
    location and bound) against the same target constraint, on the same hop
    at the same site. -/
def sameFinding (a b : ResultEntry) : Bool :=
  a.severity == b.severity &&
  a.guaranteeFile == b.guaranteeFile && a.guaranteeLine == b.guaranteeLine &&
  a.target.file == b.target.file && a.target.line == b.target.line &&
  a.target.name == b.target.name && a.hop == b.hop &&
  a.site.file == b.site.file && a.site.line == b.site.line &&
  a.sourceGuarantee == b.sourceGuarantee &&
  a.targetRequirement == b.targetRequirement &&
  a.suggestion == b.suggestion

/-- One deduplication step: add `r`, or, if `acc` already has the same
    finding, keep whichever of the two has the shorter path. -/
def dedupeStep (acc : List ResultEntry) (r : ResultEntry) : List ResultEntry :=
  if acc.any (sameFinding · r) then
    acc.map fun e => if sameFinding e r && r.path.length < e.path.length then r else e
  else
    acc ++ [r]

/-- Report each inconsistency once. When several paths reach the same finding
    (e.g. a caller's `flows_to` edge prefixed to the path from the function that
    owns the constraint), keep the shortest path, which starts at the owner. -/
def dedupeResults (results : List ResultEntry) : List ResultEntry :=
  results.foldl dedupeStep []

/-- Add one raw search result's entries (`viewPath`) to the deduplicated
    report. -/
def reportStep (acc : List ResultEntry) (x : List Edge × List CheckResult) : List ResultEntry :=
  (collectEntries (viewPath x).1 (viewPath x).2).foldl dedupeStep acc

/-- The deduplicated report of raw search results, built path by path (the
    same entries as `dedupeResults (collectResults (raw.map viewPath))`, without
    holding every path's entries at once). -/
def reportRaw (raw : List (List Edge × List CheckResult)) : List ResultEntry :=
  raw.foldl reportStep []

/-- Count total contracts across all nodes. -/
def countContracts (nodes : List Node) : Nat :=
  nodes.foldl (fun acc n => acc + n.preconditions.length + n.postconditions.length) 0

/-- Default for `--max-paths`. -/
def defaultMaxPaths : Nat := 200000

/-- Edge traversals the budget check allows for a path budget. -/
def stepBudget (maxPaths : Nat) : Nat := 64 * maxPaths + 100000

/-- Run the full search and check with a prepared setup. -/
def runCheckerWith (s : SearchSetup) (graph : ContractGraph) : CheckOutput :=
  -- streaming: (paths seen, report so far), one path at a time
  let r := foldRaw s graph (fun (acc : Nat × List ResultEntry) x => (acc.1 + 1, reportStep acc.2 x))
    (0, [])
  let results := r.2
  let hasErrors := results.any (·.severity == "error")
  { summary := {
      contractsChecked := countContracts graph.nodes
      edgesChecked := graph.edges.length
      pathsChecked := r.1
    }
    results := results
    exitCode := if hasErrors then 1 else 0
  }

/-- The output when the path budget is exceeded: nothing was checked; one
    result of severity error, status "incomplete"; exit code 2. -/
def incompleteOutput (graph : ContractGraph) (maxPaths : Nat) (stepsExceeded : Bool) :
    CheckOutput :=
  let what := if stepsExceeded then
      s!"the path search exceeded {stepBudget maxPaths} steps"
    else s!"the graph has more than {maxPaths} data paths"
  { summary := {
      contractsChecked := countContracts graph.nodes
      edgesChecked := graph.edges.length
      pathsChecked := 0
    }
    results := [{
      status := "incomplete"
      severity := "error"
      source := { file := "", line := 0, name := "" }
      target := { file := "", line := 0, name := "" }
      path := []
      sourceGuarantee := ""
      targetRequirement := ""
      verificationLevel := ""
      suggestion := s!"Path budget exceeded (--max-paths {maxPaths}): {what}. " ++
        "No path was checked. Raise --max-paths, or check a smaller part of the project." }]
    exitCode := 2
  }

/-- Run the full checking pipeline on a contract graph. If the search would
    visit more than `maxPaths` data paths (or more than `stepBudget maxPaths`
    edges), stop and report the run incomplete (exit code 2). -/
def runChecker (graph : ContractGraph) (maxPaths : Nat := defaultMaxPaths) : CheckOutput :=
  let s := searchSetup graph
  let counted := countPaths s graph maxPaths (stepBudget maxPaths)
  if counted.1 > maxPaths then incompleteOutput graph maxPaths false
  else if counted.2 > stepBudget maxPaths then incompleteOutput graph maxPaths true
  else runCheckerWith s graph

/-! ## Soundness of the executable's verdict

Exit code 0 means the budget was not exceeded and no reported entry has
severity "error". Every error `CheckResult` of every checked path yields such
an entry, and deduplication never removes the last entry of a given severity.
Hence exit code 0 implies no path has an error result, and
`checkPath_sound_noErrors` gives `stepwiseSound` for every path the search
emits, which is every data path (`checkAllPaths_complete`). -/

theorem runCheckerWith_exitCode_eq_zero_iff (s : SearchSetup) (g : ContractGraph) :
    (runCheckerWith s g).exitCode = 0 ↔
      ∀ e ∈ (runCheckerWith s g).results, e.severity ≠ "error" := by
  simp only [runCheckerWith]
  split
  · rename_i hany
    obtain ⟨e, he, hs⟩ := List.any_eq_true.mp hany
    simp only [beq_iff_eq] at hs
    exact ⟨fun h => absurd h (by decide), fun h => absurd hs (h e he)⟩
  · rename_i hany
    refine ⟨fun _ e he hs => hany (List.any_eq_true.mpr ⟨e, he, by simp [hs]⟩), fun _ => rfl⟩

/-- An exceeded budget exits with code 2. -/
theorem incompleteOutput_exitCode (g : ContractGraph) (m : Nat) (b : Bool) :
    (incompleteOutput g m b).exitCode = 2 := rfl

/-- Exit code 0 means the budget was not exceeded: the full search ran. -/
theorem runChecker_eq_of_exitCode_zero (g : ContractGraph) (maxPaths : Nat)
    (h : (runChecker g maxPaths).exitCode = 0) :
    runChecker g maxPaths = runCheckerWith (searchSetup g) g := by
  unfold runChecker at h ⊢
  simp only at h ⊢
  split at h
  · simp [incompleteOutput] at h
  · split at h
    · simp [incompleteOutput] at h
    · rename_i h1 h2
      rw [if_neg h1, if_neg h2]

/-- Exit code 0 iff no reported entry has severity "error" (an incomplete
    run reports one). -/
theorem runChecker_exitCode_eq_zero_iff (g : ContractGraph) (maxPaths : Nat := defaultMaxPaths) :
    (runChecker g maxPaths).exitCode = 0 ↔
      ∀ e ∈ (runChecker g maxPaths).results, e.severity ≠ "error" := by
  unfold runChecker
  simp only
  split
  · simp [incompleteOutput]
  · split
    · simp [incompleteOutput]
    · exact runCheckerWith_exitCode_eq_zero_iff _ _

/-- Entries that are the same finding have the same severity. -/
theorem sameFinding_severity {a b : ResultEntry} (h : sameFinding a b = true) :
    a.severity = b.severity := by
  unfold sameFinding at h
  simp only [Bool.and_eq_true, beq_iff_eq] at h
  exact h.1.1.1.1.1.1.1.1.1.1.1

/-- A dedupe step keeps an entry of every severity already present. -/
theorem dedupeStep_keeps (acc : List ResultEntry) (r : ResultEntry) (s : String)
    (h : ∃ x ∈ acc, x.severity = s) : ∃ y ∈ dedupeStep acc r, y.severity = s := by
  obtain ⟨x, hx, hs⟩ := h
  unfold dedupeStep
  split
  · refine ⟨if sameFinding x r && r.path.length < x.path.length then r else x,
            List.mem_map.mpr ⟨x, hx, rfl⟩, ?_⟩
    split
    · rename_i hc
      simp only [Bool.and_eq_true] at hc
      rw [← sameFinding_severity hc.1]; exact hs
    · exact hs
  · exact ⟨x, List.mem_append_left _ hx, hs⟩

/-- A dedupe step keeps an entry with the new entry's severity. -/
theorem dedupeStep_adds (acc : List ResultEntry) (r : ResultEntry) (s : String)
    (h : r.severity = s) : ∃ y ∈ dedupeStep acc r, y.severity = s := by
  by_cases hany : acc.any (sameFinding · r) = true
  · obtain ⟨x, hx, hsf⟩ := List.any_eq_true.mp hany
    exact dedupeStep_keeps acc r s ⟨x, hx, (sameFinding_severity hsf).trans h⟩
  · unfold dedupeStep
    rw [if_neg hany]
    exact ⟨r, List.mem_append_right _ (List.mem_singleton_self r), h⟩

theorem foldl_dedupeStep_keeps (rs acc : List ResultEntry) (s : String)
    (h : ∃ x ∈ acc, x.severity = s) : ∃ y ∈ rs.foldl dedupeStep acc, y.severity = s := by
  induction rs generalizing acc with
  | nil => exact h
  | cons hd tl ih => exact ih _ (dedupeStep_keeps acc hd s h)

/-- Folding dedupe steps from any start keeps an entry of every severity of
    its input. -/
theorem foldl_dedupeStep_finds (rs : List ResultEntry) (e : ResultEntry) (he : e ∈ rs) :
    ∀ acc, ∃ y ∈ rs.foldl dedupeStep acc, y.severity = e.severity := by
  induction rs with
  | nil => cases he
  | cons hd tl ih =>
    intro acc
    rcases List.mem_cons.mp he with rfl | htl
    · exact foldl_dedupeStep_keeps tl _ _ (dedupeStep_adds acc e _ rfl)
    · exact ih htl _

/-- Deduplication keeps at least one entry of every severity in its input. -/
theorem dedupeResults_keeps (rs : List ResultEntry) (e : ResultEntry) (he : e ∈ rs) :
    ∃ y ∈ dedupeResults rs, y.severity = e.severity :=
  foldl_dedupeStep_finds rs e he []

/-- Every error result of a checked path yields an entry of severity "error". -/
theorem collectEntries_error (path : List Edge) (rs : List CheckResult) (r : CheckResult)
    (hr : r ∈ rs) (herr : r.isError = true) :
    ∃ e ∈ collectEntries path rs, e.severity = "error" := by
  cases r with
  | consistent => cases herr
  | inconsistent d =>
    have hsev : d.severity = .error := by
      cases hd : d.severity
      · rfl
      · simp [CheckResult.isError, hd] at herr
    unfold collectEntries
    split
    · rename_i hall
      have := List.all_eq_true.mp hall _ hr
      simp [CheckResult.isConsistent] at this
    · refine ⟨_, List.mem_filterMap.mpr ⟨_, hr, rfl⟩, ?_⟩
      simp [buildResultEntry, hsev, toString]

/-- Every error result of a checked path is reported with severity "error". -/
theorem collectResults_error (pathResults : List (List Edge × List CheckResult))
    (p : List Edge) (rs : List CheckResult) (r : CheckResult)
    (hmem : (p, rs) ∈ pathResults) (hr : r ∈ rs) (herr : r.isError = true) :
    ∃ e ∈ collectResults pathResults, e.severity = "error" := by
  obtain ⟨e, he, hs⟩ := collectEntries_error p rs r hr herr
  exact ⟨e, List.mem_flatMap.mpr ⟨(p, rs), hmem, he⟩, hs⟩

theorem foldl_reportStep_keeps (raw : List (List Edge × List CheckResult))
    (acc : List ResultEntry) (s : String) (h : ∃ x ∈ acc, x.severity = s) :
    ∃ y ∈ raw.foldl reportStep acc, y.severity = s := by
  induction raw generalizing acc with
  | nil => exact h
  | cons hd tl ih => exact ih _ (foldl_dedupeStep_keeps _ acc s h)

/-- Every error result of every raw search result is reported with severity
    "error". -/
theorem reportRaw_error (raw : List (List Edge × List CheckResult))
    (x : List Edge × List CheckResult) (hx : x ∈ raw) (r : CheckResult)
    (hr : r ∈ (viewPath x).2) (herr : r.isError = true) :
    ∃ e ∈ reportRaw raw, e.severity = "error" := by
  obtain ⟨e, he, hs⟩ := collectEntries_error (viewPath x).1 _ r hr herr
  suffices ∀ acc, ∃ y ∈ raw.foldl reportStep acc, y.severity = "error" from this []
  induction raw with
  | nil => cases hx
  | cons hd tl ih =>
    intro acc
    rcases List.mem_cons.mp hx with rfl | htl
    · obtain ⟨y, hy, hys⟩ := foldl_dedupeStep_finds _ e he acc
      exact foldl_reportStep_keeps tl _ _ ⟨y, hy, hys.trans hs⟩
    · exact ih htl _

theorem foldl_count_report (raw : List (List Edge × List CheckResult)) (n : Nat)
    (acc : List ResultEntry) :
    raw.foldl (fun (acc : Nat × List ResultEntry) x => (acc.1 + 1, reportStep acc.2 x)) (n, acc) =
      (n + raw.length, raw.foldl reportStep acc) := by
  induction raw generalizing n acc with
  | nil => rfl
  | cons hd tl ih =>
    simp only [List.foldl_cons, ih, List.length_cons]
    congr 1; omega

/-- The streamed report is `reportRaw` of the search results. -/
theorem runCheckerWith_results (s : SearchSetup) (g : ContractGraph) :
    (runCheckerWith s g).results = reportRaw (searchRaw s g) := by
  simp only [runCheckerWith, foldRaw_eq, foldl_count_report, reportRaw]

/-- The number of checked paths is the number of search results. -/
theorem runCheckerWith_paths (s : SearchSetup) (g : ContractGraph) :
    (runCheckerWith s g).summary.pathsChecked = (searchRaw s g).length := by
  simp only [runCheckerWith, foldRaw_eq, foldl_count_report, Nat.zero_add]

/-- With exit code 0, no result of any checked path is an error. -/
theorem runChecker_noErrors (g : ContractGraph) {maxPaths : Nat}
    (h : (runChecker g maxPaths).exitCode = 0) :
    ∀ y ∈ checkAllPaths g, ∀ r ∈ y.2, r.isError = false := by
  have heq := runChecker_eq_of_exitCode_zero g maxPaths h
  rw [heq] at h
  intro y hy r hr
  cases herr : r.isError with
  | false => rfl
  | true =>
    exfalso
    obtain ⟨x, hx, rfl⟩ := List.mem_map.mp hy
    obtain ⟨e, he, hs⟩ := reportRaw_error _ x hx r hr herr
    rw [← runCheckerWith_results] at he
    exact (runCheckerWith_exitCode_eq_zero_iff _ g).mp h e he hs

/-- END-TO-END SOUNDNESS. If the checker exits with code 0 (warnings allowed;
    so the path budget was not exceeded), every enumerated path is stepwise
    sound: at every hop, the (composed) source guarantees imply the target's
    requirements. -/
theorem runChecker_sound (g : ContractGraph) {maxPaths : Nat}
    (h : (runChecker g maxPaths).exitCode = 0) :
    ∀ p ∈ enumeratePaths g, p ≠ [] → stepwiseSound p := by
  intro p hp hne
  obtain ⟨y, hy, rfl⟩ := List.mem_map.mp hp
  apply checkPath_sound_noErrors _ hne
  rw [← checkAllPaths_spec g y hy]
  exact runChecker_noErrors g h y hy

/-- END-TO-END SOUNDNESS over all data paths. If the checker exits with code 0,
    every checked data path of the graph (`IsDataPath`: simple, non-`calls`
    edges, function node to model node) is stepwise sound — not only the ones
    the enumeration happened to produce (`enumeratePaths_complete`). -/
theorem runChecker_sound_all (g : ContractGraph) {maxPaths : Nat}
    (h : (runChecker g maxPaths).exitCode = 0) :
    ∀ p, IsDataPath g p → stepwiseSound p :=
  fun p hp => runChecker_sound g h p (enumeratePaths_complete g p hp) hp.1

/-- Parse the arguments after the database path. -/
def parseOptions : List String → Except String Nat
  | [] => .ok defaultMaxPaths
  | ["--max-paths", n] =>
    match n.toNat? with
    | some k => .ok k
    | none => .error s!"--max-paths expects a non-negative integer, got '{n}'"
  | args => .error s!"unexpected arguments: {" ".intercalate args}"

/-- Main entry point. -/
def main (args : List String) : IO UInt32 := do
  match args with
  | [] =>
    IO.eprintln "Usage: contract-graph-checker <database.sqlite> [--max-paths N]"
    IO.eprintln "  (No database path provided)"
    return 2
  | dbPath :: rest =>
    match parseOptions rest with
    | .error msg =>
      IO.eprintln s!"Error: {msg}"
      IO.eprintln "Usage: contract-graph-checker <database.sqlite> [--max-paths N]"
      return 2
    | .ok maxPaths =>
      -- Check if the database file exists
      let dbFile : System.FilePath := ⟨dbPath⟩
      let fileExists ← dbFile.pathExists
      if !fileExists then
        IO.eprintln s!"Error: database file not found: {dbPath}"
        return 2
      else do
        -- Read the contract graph from SQLite
        let graph ← readContractGraph dbPath
        -- Run the checker pipeline
        let output := runChecker graph maxPaths
        -- Output JSON to stdout
        IO.println (outputToJson output)
        return output.exitCode.toUInt32

end ContractGraph

def main := ContractGraph.main
