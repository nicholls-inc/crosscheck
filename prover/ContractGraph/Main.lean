-- Main.lean

import ContractGraph.Types
import ContractGraph.BehaviorModel
import ContractGraph.DependentExpr
import ContractGraph.Translation
import ContractGraph.Checker
import ContractGraph.Composition
import ContractGraph.Diagnostics
import ContractGraph.Search
import ContractGraph.StateSearch

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

/-- Where the violating guarantee comes from (`"guarantee_at"`): for an
    error, the source constraint's origin (for a bound composed from a
    dependent expression, the input constraint whose value it equals), else
    its own location; for a warning, the target's requirement location (a
    warning has no violating guarantee). -/
def guaranteeAt (r : ResultEntry) : SiteLocation :=
  if r.severity == "error" then { file := r.guaranteeAtFile, line := r.guaranteeAtLine }
  else { file := r.target.file, line := r.target.line }

/-- Format a ResultEntry as JSON. -/
def resultEntryToJson (r : ResultEntry) : String :=
  s!"\{\"status\": \"{r.status}\", " ++
  s!"\"severity\": \"{r.severity}\", " ++
  s!"\"source\": {sourceLocationToJson r.source}, " ++
  s!"\"target\": {sourceLocationToJson r.target}, " ++
  s!"\"path\": {stringListToJson r.path}, " ++
  s!"\"witness\": {stringListToJson r.witness}, " ++
  s!"\"hop\": {stringListToJson r.hop}, " ++
  s!"\"site\": {siteLocationToJson r.site}, " ++
  s!"\"guarantee_at\": {siteLocationToJson (guaranteeAt r)}, " ++
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
  s!"\"paths_checked\": {output.summary.pathsChecked}, " ++
  s!"\"states_checked\": {output.summary.statesChecked}}, " ++
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

/-- The prefix of a witness path up to and including a diagnostic's hop:
    the first `n` edges when the hop's position `n` is known (the state
    checker), else up to the first edge with the hop's source and target
    names and site; the whole path when none matches (an untagged
    diagnostic). -/
def hopPrefix (path : List Edge) (d : DiagnosticInfo) (hopLen : Option Nat) : List Edge :=
  match hopLen with
  | some n => path.take n
  | none =>
    match path.findIdx? (fun e => e.source.name == d.hopSource && e.target.name == d.hopTarget &&
        e.siteFile == d.siteFile && e.siteLine == d.siteLine) with
    | some i => path.take (i + 1)
    | none => path

/-- The result entries of one checked path: one per inconsistent result.
    Each entry's `path` ends at its failing hop's target (`hopPrefix`);
    `witness` is the whole path. -/
def collectEntries (path : List Edge) (results : List CheckResult) (hopLen : Option Nat := none) :
    List ResultEntry :=
  if results.all (·.isConsistent) then [] else
  let witness := pathNamesOf path
  let head := path.head?.map (·.source)
  results.filterMap fun r =>
    match r with
    | .consistent => none
    | .inconsistent diag =>
      let reported := hopPrefix path diag hopLen
      let pathNames := pathNamesOf reported
      some { buildResultEntry { diag with path := pathNames } pathNames
               (pathVerificationLevel reported) head with witness := witness }

/-- Collect all inconsistencies from path check results. -/
def collectResults (pathResults : List (List Edge × List CheckResult))
    : List ResultEntry :=
  pathResults.flatMap fun (path, results) => collectEntries path results

/-- Two entries report the same inconsistency when everything except the
    path (and so the path head) matches: the same source constraint (by
    location and bound) against the same target constraint, on the same hop
    at the same site. The display origin of a composed bound
    (`guaranteeAtFile/Line`) is not compared: two upstream origins that
    produce the same bound on the same hop are one finding, reported with the
    kept (shortest) path's origin. -/
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

/-- A raw search result's results as reported: without warnings when the
    path's head is a suffix head (`supp`, see `suffixHeadSet`). -/
def reportedResults (supp : Nat → Bool) (x : List Edge × List CheckResult) : List CheckResult :=
  match (viewPath x).1.head? with
  | some e => if supp e.source.id then dropWarnings (viewPath x).2 else (viewPath x).2
  | none => (viewPath x).2

/-- Reporting keeps every error. -/
theorem mem_reportedResults (supp : Nat → Bool) (x : List Edge × List CheckResult)
    (r : CheckResult) (hr : r ∈ (viewPath x).2) (herr : r.isError = true) :
    r ∈ reportedResults supp x := by
  unfold reportedResults
  split
  · split
    · exact mem_dropWarnings _ r hr herr
    · exact hr
  · exact hr

/-- Add one raw search result's entries (`viewPath`, `reportedResults`) to
    the deduplicated report. -/
def reportStep (supp : Nat → Bool) (acc : List ResultEntry) (x : List Edge × List CheckResult) :
    List ResultEntry :=
  (collectEntries (viewPath x).1 (reportedResults supp x)).foldl dedupeStep acc

/-- The deduplicated report of raw search results, built path by path (the
    same entries as `dedupeResults (collectResults (raw.map viewPath))` when
    nothing is suppressed, without holding every path's entries at once). -/
def reportRaw (supp : Nat → Bool) (raw : List (List Edge × List CheckResult)) : List ResultEntry :=
  raw.foldl (reportStep supp) []

/-- Count total contracts across all nodes. -/
def countContracts (nodes : List Node) : Nat :=
  nodes.foldl (fun acc n => acc + n.preconditions.length + n.postconditions.length) 0

/-- Default for `--max-paths`. -/
def defaultMaxPaths : Nat := 200000

/-- Edge traversals the budget check allows for a path budget. -/
def stepBudget (maxPaths : Nat) : Nat := 64 * maxPaths + 100000

/-- Run the full search and check with a prepared setup. -/
def runCheckerWith (s : SearchSetup) (graph : ContractGraph) : CheckOutput :=
  let supp := suffixHeadSet graph
  -- streaming: (paths seen, report so far), one path at a time
  let r := foldRaw s graph
    (fun (acc : Nat × List ResultEntry) x => (acc.1 + 1, reportStep supp.contains acc.2 x)) (0, [])
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

/-- The path-based checker (the executable's before the state-based
    `runChecker`; kept as a reference for tests): check every data path. If
    the search would visit more than `maxPaths` data paths (or more than
    `stepBudget maxPaths` edges), stop and report the run incomplete (exit
    code 2). -/
def runCheckerPaths (graph : ContractGraph) (maxPaths : Nat := defaultMaxPaths) : CheckOutput :=
  let s := searchSetup graph
  let counted := countPaths s graph maxPaths (stepBudget maxPaths)
  if counted.1 > maxPaths then incompleteOutput graph maxPaths false
  else if counted.2 > stepBudget maxPaths then incompleteOutput graph maxPaths true
  else runCheckerWith s graph

/-! ## Soundness of the path-based checker's verdict

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
theorem runCheckerPaths_eq_of_exitCode_zero (g : ContractGraph) (maxPaths : Nat)
    (h : (runCheckerPaths g maxPaths).exitCode = 0) :
    runCheckerPaths g maxPaths = runCheckerWith (searchSetup g) g := by
  unfold runCheckerPaths at h ⊢
  simp only at h ⊢
  split at h
  · simp [incompleteOutput] at h
  · split at h
    · simp [incompleteOutput] at h
    · rename_i h1 h2
      rw [if_neg h1, if_neg h2]

/-- Exit code 0 iff no reported entry has severity "error" (an incomplete
    run reports one). -/
theorem runCheckerPaths_exitCode_eq_zero_iff (g : ContractGraph) (maxPaths : Nat := defaultMaxPaths) :
    (runCheckerPaths g maxPaths).exitCode = 0 ↔
      ∀ e ∈ (runCheckerPaths g maxPaths).results, e.severity ≠ "error" := by
  unfold runCheckerPaths
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
    (hr : r ∈ rs) (herr : r.isError = true) (hopLen : Option Nat := none) :
    ∃ e ∈ collectEntries path rs hopLen, e.severity = "error" := by
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

theorem foldl_reportStep_keeps (supp : Nat → Bool) (raw : List (List Edge × List CheckResult))
    (acc : List ResultEntry) (s : String) (h : ∃ x ∈ acc, x.severity = s) :
    ∃ y ∈ raw.foldl (reportStep supp) acc, y.severity = s := by
  induction raw generalizing acc with
  | nil => exact h
  | cons hd tl ih => exact ih _ (foldl_dedupeStep_keeps _ acc s h)

/-- Every error result of every raw search result is reported with severity
    "error". -/
theorem reportRaw_error (supp : Nat → Bool) (raw : List (List Edge × List CheckResult))
    (x : List Edge × List CheckResult) (hx : x ∈ raw) (r : CheckResult)
    (hr : r ∈ (viewPath x).2) (herr : r.isError = true) :
    ∃ e ∈ reportRaw supp raw, e.severity = "error" := by
  obtain ⟨e, he, hs⟩ := collectEntries_error (viewPath x).1 _ r
    (mem_reportedResults supp x r hr herr) herr
  suffices ∀ acc, ∃ y ∈ raw.foldl (reportStep supp) acc, y.severity = "error" from this []
  induction raw with
  | nil => cases hx
  | cons hd tl ih =>
    intro acc
    rcases List.mem_cons.mp hx with rfl | htl
    · obtain ⟨y, hy, hys⟩ := foldl_dedupeStep_finds _ e he acc
      exact foldl_reportStep_keeps _ tl _ _ ⟨y, hy, hys.trans hs⟩
    · exact ih htl _

theorem foldl_count_report (supp : Nat → Bool) (raw : List (List Edge × List CheckResult))
    (n : Nat) (acc : List ResultEntry) :
    raw.foldl (fun (acc : Nat × List ResultEntry) x => (acc.1 + 1, reportStep supp acc.2 x))
      (n, acc) = (n + raw.length, raw.foldl (reportStep supp) acc) := by
  induction raw generalizing n acc with
  | nil => rfl
  | cons hd tl ih =>
    simp only [List.foldl_cons, ih, List.length_cons]
    congr 1; omega

/-- The streamed report is `reportRaw` of the search results. -/
theorem runCheckerWith_results (s : SearchSetup) (g : ContractGraph) :
    (runCheckerWith s g).results = reportRaw (suffixHeadSet g).contains (searchRaw s g) := by
  simp only [runCheckerWith, foldRaw_eq, foldl_count_report, reportRaw]

/-- The number of checked paths is the number of search results. -/
theorem runCheckerWith_paths (s : SearchSetup) (g : ContractGraph) :
    (runCheckerWith s g).summary.pathsChecked = (searchRaw s g).length := by
  simp only [runCheckerWith, foldRaw_eq, foldl_count_report, Nat.zero_add]

/-- With exit code 0, no result of any checked path is an error. -/
theorem runCheckerPaths_noErrors (g : ContractGraph) {maxPaths : Nat}
    (h : (runCheckerPaths g maxPaths).exitCode = 0) :
    ∀ y ∈ checkAllPaths g, ∀ r ∈ y.2, r.isError = false := by
  have heq := runCheckerPaths_eq_of_exitCode_zero g maxPaths h
  rw [heq] at h
  intro y hy r hr
  cases herr : r.isError with
  | false => rfl
  | true =>
    exfalso
    obtain ⟨x, hx, rfl⟩ := List.mem_map.mp hy
    obtain ⟨e, he, hs⟩ := reportRaw_error _ _ x hx r hr herr
    rw [← runCheckerWith_results] at he
    exact (runCheckerWith_exitCode_eq_zero_iff _ g).mp h e he hs

/-- END-TO-END SOUNDNESS. If the checker exits with code 0 (warnings allowed;
    so the path budget was not exceeded), every enumerated path is stepwise
    sound: at every hop, the (composed) source guarantees imply the target's
    requirements. -/
theorem runCheckerPaths_sound (g : ContractGraph) {maxPaths : Nat}
    (h : (runCheckerPaths g maxPaths).exitCode = 0) :
    ∀ p ∈ enumeratePaths g, p ≠ [] → stepwiseSound p := by
  intro p hp hne
  obtain ⟨y, hy, rfl⟩ := List.mem_map.mp hp
  apply checkPath_sound_noErrors _ hne
  rw [← checkAllPaths_spec g y hy]
  exact runCheckerPaths_noErrors g h y hy

/-- END-TO-END SOUNDNESS over all data paths. If the checker exits with code 0,
    every checked data path of the graph (`IsDataPath`: simple, non-`calls`
    edges, function node to model node) is stepwise sound — not only the ones
    the enumeration happened to produce (`enumeratePaths_complete`). -/
theorem runCheckerPaths_sound_all (g : ContractGraph) {maxPaths : Nat}
    (h : (runCheckerPaths g maxPaths).exitCode = 0) :
    ∀ p, IsDataPath g p → stepwiseSound p :=
  fun p hp => runCheckerPaths_sound g h p (enumeratePaths_complete g p hp) hp.1


/-! ## The state-based checker (the executable's)

`runChecker` explores the composed hops reachable from the first hops of
paths (`explore`, StateSearch.lean), checks the exploration is closed
(`closedStates`), and checks each hop once (`checkHop`). Each finding is
reported once (deduplicated as by `dedupeResults`, keeping the shortest
witness path), with a witness data path: the shortest walk found to the hop,
then a shortest continuation to a model node. -/

/-- Default for `--max-states` (alias `--max-paths`): distinct hop states in
    total. -/
def defaultMaxStates : Nat := 2000000

/-- Default for `--max-states-per-edge`: distinct composed states on one
    edge. Only a cycle through a dependent postcondition that keeps changing
    a bound gets near it. -/
def defaultMaxPerEdge : Nat := 64

/-- Deduplicated report under construction: the entries in first-seen order,
    and an index from a finding key to an entry's position. The index is a
    hint: a hit is confirmed with `sameFinding`, a miss appends. -/
structure Dedupe where
  arr : Array ResultEntry := #[]
  idx : Std.HashMap String Nat := ∅

/-- The fields `sameFinding` compares, as one key. -/
def findingKey (r : ResultEntry) : String :=
  "\x1f".intercalate [r.severity, r.guaranteeFile, toString r.guaranteeLine, r.target.file,
    toString r.target.line, r.target.name, toString r.hop, r.site.file, toString r.site.line,
    r.sourceGuarantee, r.targetRequirement, r.suggestion]

def Dedupe.push (d : Dedupe) (r : ResultEntry) : Dedupe :=
  { arr := d.arr.push r, idx := d.idx.insert (findingKey r) d.arr.size }

/-- `dedupeStep` with an index: add `r`, or, if the report already has the
    same finding, keep whichever of the two has the shorter path. -/
def Dedupe.step (d : Dedupe) (r : ResultEntry) : Dedupe :=
  match d.idx[findingKey r]? with
  | some i =>
    if h : i < d.arr.size then
      if sameFinding d.arr[i] r then
        if r.path.length < d.arr[i].path.length then { d with arr := d.arr.set i r h } else d
      else d.push r
    else d.push r
  | none => d.push r

/-- `collectEntries`, computing the path and the hop's position (`path ()`)
    only when some result is inconsistent. -/
def guardedEntries (path : Unit → List Edge × Nat) (rs : List CheckResult) : List ResultEntry :=
  if rs.all (·.isConsistent) then [] else
    let p := path ()
    collectEntries p.1 rs (some p.2)

theorem guardedEntries_error (path : Unit → List Edge × Nat) (rs : List CheckResult) (x : CheckResult)
    (hx : x ∈ rs) (herr : x.isError = true) :
    ∃ e ∈ guardedEntries path rs, e.severity = "error" := by
  unfold guardedEntries
  split
  · rename_i hall
    have := List.all_eq_true.mp hall x hx
    cases x with
    | consistent => cases herr
    | inconsistent _ => simp [CheckResult.isConsistent] at this
  · exact collectEntries_error _ _ x hx herr _

/-- The report entries of one state: those of its hop's results (without
    the warnings unless `warn`, see `warnFlags`), with the state's witness
    path. -/
def stateEntries (st : StateSetup) (ex : Explored) (r : StateRec) (warn : Bool) :
    List ResultEntry :=
  guardedEntries (fun _ => witnessWithHop st ex r)
    (if warn then checkHop r.hop else dropWarnings (checkHop r.hop))

/-- The deduplicated report of every explored state; `flags[i]` says whether
    state `i` reports its warnings. -/
def reportStatesWith (st : StateSetup) (ex : Explored) (flags : Array Bool) : List ResultEntry :=
  (ex.recs.toList.zipIdx.foldl (fun d (r, i) => (stateEntries st ex r (flags.getD i true)).foldl
    Dedupe.step d) {}).arr.toList

/-- The report of a graph's states: warnings only from states reached from a
    head that is not a call-site suffix head. -/
def reportStates (st : StateSetup) (g : ContractGraph) (ex : Explored) : List ResultEntry :=
  reportStatesWith st ex (warnFlags st ex (suffixHeadSet g))

/-- The output of a finished, closed exploration. `paths_checked` counts the
    checked hop states. -/
def runStatesWith (st : StateSetup) (graph : ContractGraph) (ex : Explored) : CheckOutput :=
  let results := reportStates st graph ex
  { summary := {
      contractsChecked := countContracts graph.nodes
      edgesChecked := graph.edges.length
      pathsChecked := ex.recs.size
      statesChecked := ex.recs.size
    }
    results := results
    exitCode := if results.any (·.severity == "error") then 1 else 0 }

/-- An incomplete run: nothing reported but one result of severity error,
    status "incomplete", with `msg`; exit code 2. -/
def incompleteWith (graph : ContractGraph) (msg : String) : CheckOutput :=
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
      suggestion := msg }]
    exitCode := 2
  }

/-- The last `n` elements of a list. -/
def lastN {α : Type} (n : Nat) (xs : List α) : List α := xs.drop (xs.length - n)

/-- Why a state cap was exceeded: the hop, its site, the walk that reached
    the new state (with the cycle it went round, when it repeats a node) and
    the composed bounds of the hop's source. -/
def capMessage (ex : Explored) (maxPerEdge : Nat) (hop : Edge) (pred : Nat) (raw : Edge) :
    String :=
  let walk := prefixOf ex.recs ⟨hop, pred, raw⟩
  let names := pathNamesOf walk
  let ids := walk.map (·.source.id) ++ [raw.target.id]
  -- the cycle: from the last earlier visit of the hop's target to the end
  let cycle :=
    match (ids.dropLast.zip names.dropLast).reverse.findIdx? (·.1 == raw.target.id) with
    | some k => lastN (k + 2) names
    | none => []
  let site := if raw.siteFile.isEmpty then "" else s!" (site {raw.siteFile}:{raw.siteLine})"
  let bounds := ", ".intercalate (hop.source.postconditions.map formatBound)
  let shown := lastN 12 names
  s!"State cap exceeded (--max-states-per-edge {maxPerEdge}): the edge " ++
  s!"{raw.source.name} -> {raw.target.name}{site} reached more than {maxPerEdge} distinct " ++
  s!"composed states. " ++
  (if cycle.isEmpty then "" else s!"Cycle: {" -> ".intercalate cycle}. ") ++
  s!"Walk (last {shown.length} nodes): {" -> ".intercalate shown}; composed source " ++
  s!"bounds: [{bounds}]. " ++
  (if cycle.isEmpty then "Upstream paths give this edge many different composed bounds. "
   else "A dependent postcondition on this cycle keeps changing a bound. ") ++
  "Nothing was reported. Raise --max-states-per-edge, or check a smaller part of the project."

/-- Run the full checking pipeline on a contract graph (state-based). If the
    exploration exceeds `maxStates` states, or `maxPerEdge` states on one
    edge, stop and report the run incomplete (exit code 2). -/
def runChecker (graph : ContractGraph) (maxStates : Nat := defaultMaxStates)
    (maxPerEdge : Nat := defaultMaxPerEdge) : CheckOutput :=
  let st := stateSetup graph
  match explore st graph maxStates maxPerEdge with
  | .done ex =>
    if closedStates st.setup graph ex then runStatesWith st graph ex
    else incompleteWith graph
      "Internal error: the state exploration is not closed under successors. Nothing was reported."
  | .tooMany ex =>
    incompleteWith graph <|
      s!"State budget exceeded (--max-states {maxStates}): the exploration reached more than " ++
      s!"{maxStates} distinct hop states ({ex.recs.size} explored). Nothing was reported. " ++
      "Raise --max-states (alias --max-paths), or check a smaller part of the project."
  | .capped ex hop pred raw => incompleteWith graph (capMessage ex maxPerEdge hop pred raw)

/-! ### Soundness of the state-based checker

Exit code 0 means the exploration finished, passed the closure check, and no
reported entry has severity "error". Every error result of every explored
state's `checkHop` yields such an entry (`reportStates_error`: the indexed
deduplication never drops the last entry of a severity). The closure check
alone gives that every `checkPath` result of every data path is a `checkHop`
result of an explored state (`closedStates_checkPath`), whatever the
unverified exploration did. Hence no data path has an error result, and
`checkPath_sound_noErrors` gives `stepwiseSound`. The witness paths and the
exploration order play no part. -/

theorem runStatesWith_exitCode_eq_zero_iff (st : StateSetup) (g : ContractGraph) (ex : Explored) :
    (runStatesWith st g ex).exitCode = 0 ↔
      ∀ e ∈ (runStatesWith st g ex).results, e.severity ≠ "error" := by
  simp only [runStatesWith]
  split
  · rename_i hany
    obtain ⟨e, he, hs⟩ := List.any_eq_true.mp hany
    simp only [beq_iff_eq] at hs
    exact ⟨fun h => absurd h (by decide), fun h => absurd hs (h e he)⟩
  · rename_i hany
    refine ⟨fun _ e he hs => hany (List.any_eq_true.mpr ⟨e, he, by simp [hs]⟩), fun _ => rfl⟩

/-- An incomplete run exits with code 2. -/
theorem incompleteWith_exitCode (g : ContractGraph) (m : String) :
    (incompleteWith g m).exitCode = 2 := rfl

/-- Exit code 0 iff no reported entry has severity "error" (an incomplete
    run reports one). -/
theorem runChecker_exitCode_eq_zero_iff (g : ContractGraph) (maxStates : Nat := defaultMaxStates)
    (maxPerEdge : Nat := defaultMaxPerEdge) :
    (runChecker g maxStates maxPerEdge).exitCode = 0 ↔
      ∀ e ∈ (runChecker g maxStates maxPerEdge).results, e.severity ≠ "error" := by
  unfold runChecker
  simp only
  split
  · split
    · exact runStatesWith_exitCode_eq_zero_iff _ _ _
    · simp [incompleteWith]
  · simp [incompleteWith]
  · simp [incompleteWith]

/-- Exit code 0 means the exploration finished and passed the closure check. -/
theorem runChecker_done_of_exitCode_zero (g : ContractGraph) (maxStates maxPerEdge : Nat)
    (h : (runChecker g maxStates maxPerEdge).exitCode = 0) :
    ∃ ex, closedStates (searchSetup g) g ex = true ∧
      runChecker g maxStates maxPerEdge = runStatesWith (stateSetup g) g ex := by
  unfold runChecker at h ⊢
  simp only at h ⊢
  split at h
  · rename_i ex _
    split at h
    · rename_i hc
      exact ⟨ex, hc, by simp [hc]⟩
    · simp [incompleteWith] at h
  · simp [incompleteWith] at h
  · simp [incompleteWith] at h

/-- A dedupe step keeps an entry of every severity already present. -/
theorem Dedupe.step_keeps (d : Dedupe) (r : ResultEntry) (s : String)
    (h : ∃ x ∈ d.arr.toList, x.severity = s) : ∃ y ∈ (d.step r).arr.toList, y.severity = s := by
  have hpush : ∃ y ∈ (d.push r).arr.toList, y.severity = s := by
    obtain ⟨x, hx, hs⟩ := h
    exact ⟨x, by simp [Dedupe.push, hx], hs⟩
  unfold Dedupe.step
  split
  · rename_i i _
    split
    · rename_i hi
      split
      · rename_i hsf
        split
        · obtain ⟨x, hx, hs⟩ := h
          obtain ⟨j, hj, rfl⟩ := Array.mem_iff_getElem.mp (Array.mem_toList_iff.mp hx)
          by_cases hji : j = i
          · subst hji
            refine ⟨r, Array.mem_toList_iff.mpr (Array.mem_set hi), ?_⟩
            rw [← sameFinding_severity hsf]; exact hs
          · refine ⟨d.arr[j], Array.mem_toList_iff.mpr ?_, hs⟩
            have hmem := Array.getElem_mem (xs := d.arr.set i r hi) (i := j) (by simpa using hj)
            rwa [Array.getElem_set_ne hi hj (Ne.symm hji)] at hmem
        · exact h
      · exact hpush
    · exact hpush
  · exact hpush

/-- A dedupe step keeps an entry with the new entry's severity. -/
theorem Dedupe.step_adds (d : Dedupe) (r : ResultEntry) :
    ∃ y ∈ (d.step r).arr.toList, y.severity = r.severity := by
  have hpush : ∃ y ∈ (d.push r).arr.toList, y.severity = r.severity :=
    ⟨r, by simp [Dedupe.push], rfl⟩
  unfold Dedupe.step
  split
  · rename_i i _
    split
    · rename_i hi
      split
      · rename_i hsf
        split
        · exact ⟨r, Array.mem_toList_iff.mpr (Array.mem_set hi), rfl⟩
        · exact ⟨d.arr[i], Array.mem_toList_iff.mpr (Array.getElem_mem hi),
            sameFinding_severity hsf⟩
      · exact hpush
    · exact hpush
  · exact hpush

theorem foldl_Dedupe_keeps (rs : List ResultEntry) (d : Dedupe) (s : String)
    (h : ∃ x ∈ d.arr.toList, x.severity = s) :
    ∃ y ∈ (rs.foldl Dedupe.step d).arr.toList, y.severity = s := by
  induction rs generalizing d with
  | nil => exact h
  | cons hd tl ih => exact ih _ (Dedupe.step_keeps d hd s h)

theorem foldl_Dedupe_finds (rs : List ResultEntry) (e : ResultEntry) (he : e ∈ rs) :
    ∀ d, ∃ y ∈ (rs.foldl Dedupe.step d).arr.toList, y.severity = e.severity := by
  induction rs with
  | nil => cases he
  | cons hd tl ih =>
    intro d
    rcases List.mem_cons.mp he with rfl | htl
    · exact foldl_Dedupe_keeps tl _ _ (Dedupe.step_adds d e)
    · exact ih htl _

/-- Every error result of every explored state's hop is reported with
    severity "error". -/
theorem reportStatesWith_error (st : StateSetup) (ex : Explored) (flags : Array Bool)
    (r : StateRec) (hr : r ∈ ex.recs.toList) (x : CheckResult) (hx : x ∈ checkHop r.hop)
    (herr : x.isError = true) : ∃ e ∈ reportStatesWith st ex flags, e.severity = "error" := by
  obtain ⟨i, hi⟩ := List.mem_iff_getElem?.mp hr
  have hri : (r, i) ∈ ex.recs.toList.zipIdx := List.mem_zipIdx_iff_getElem?.mpr hi
  have hent : ∃ e ∈ stateEntries st ex r (flags.getD i true), e.severity = "error" := by
    have hx' : x ∈ (if flags.getD i true then checkHop r.hop else dropWarnings (checkHop r.hop)) := by
      split
      · exact hx
      · exact mem_dropWarnings _ x hx herr
    exact guardedEntries_error _ _ x hx' herr
  obtain ⟨e, he, hs⟩ := hent
  unfold reportStatesWith
  let F := fun (d : Dedupe) (p : StateRec × Nat) =>
    (stateEntries st ex p.1 (flags.getD p.2 true)).foldl Dedupe.step d
  suffices ∀ (l : List (StateRec × Nat)), (r, i) ∈ l → ∀ d,
      ∃ y ∈ (l.foldl F d).arr.toList, y.severity = "error"
    from this _ hri {}
  intro l hl
  induction l with
  | nil => cases hl
  | cons hd tl ih =>
    intro d
    rcases List.mem_cons.mp hl with rfl | htl
    · have ⟨y, hy, hys⟩ := foldl_Dedupe_finds _ e he d
      have key : ∀ (l : List (StateRec × Nat)) d', (∃ y ∈ d'.arr.toList, y.severity = "error") →
          ∃ y ∈ (l.foldl F d').arr.toList, y.severity = "error" := by
        intro l
        induction l with
        | nil => exact fun _ h => h
        | cons a l ih' => exact fun d' h => ih' _ (foldl_Dedupe_keeps _ d' _ h)
      exact key tl _ ⟨y, hy, hys.trans hs⟩
    · exact ih htl _

/-- With exit code 0, no result of any data path's `checkPath` is an error. -/
theorem runChecker_noErrors (g : ContractGraph) {maxStates maxPerEdge : Nat}
    (h : (runChecker g maxStates maxPerEdge).exitCode = 0) :
    ∀ p, IsDataPath g p → ∀ x ∈ checkPath p, x.isError = false := by
  obtain ⟨ex, hc, heq⟩ := runChecker_done_of_exitCode_zero g maxStates maxPerEdge h
  intro p hp x hx
  cases herr : x.isError with
  | false => rfl
  | true =>
    exfalso
    obtain ⟨r, hr, hxr⟩ := closedStates_checkPath g ex hc p hp x hx
    obtain ⟨e, he, hs⟩ := reportStatesWith_error (stateSetup g) ex _ r hr x hxr herr
    rw [heq] at h
    exact (runStatesWith_exitCode_eq_zero_iff _ g ex).mp h e he hs

/-- END-TO-END SOUNDNESS over all data paths. If the checker exits with code 0
    (warnings allowed; so the state budgets were not exceeded), every checked
    data path of the graph (`IsDataPath`: simple, non-`calls` edges, function
    node to model node) is stepwise sound: at every hop, the (composed) source
    guarantees imply the target's requirements. -/
theorem runChecker_sound_all (g : ContractGraph) {maxStates maxPerEdge : Nat}
    (h : (runChecker g maxStates maxPerEdge).exitCode = 0) :
    ∀ p, IsDataPath g p → stepwiseSound p :=
  fun p hp => checkPath_sound_noErrors p hp.1 (runChecker_noErrors g h p hp)

/-- END-TO-END SOUNDNESS over the enumerated paths (every one of which is a
    data path's worth of hops: `enumeratePaths` is complete, and every data
    path is covered by `runChecker_sound_all`). -/
theorem runChecker_sound (g : ContractGraph) {maxStates maxPerEdge : Nat}
    (h : (runChecker g maxStates maxPerEdge).exitCode = 0) :
    ∀ p, IsDataPath g p → p ∈ enumeratePaths g ∧ stepwiseSound p :=
  fun p hp => ⟨enumeratePaths_complete g p hp, runChecker_sound_all g h p hp⟩

/-! ## Command line -/

/-- Checker options. -/
structure Options where
  maxStates : Nat := defaultMaxStates
  maxPerEdge : Nat := defaultMaxPerEdge
  deriving Repr, BEq

/-- Parse the arguments after the database path: `--max-states N`
    (`--max-paths N` is an alias, kept for the extractor CLI) and
    `--max-states-per-edge N`, in any order. -/
def parseOptionsFrom (o : Options) : List String → Except String Options
  | [] => .ok o
  | flag :: n :: rest =>
    if flag == "--max-states" || flag == "--max-paths" || flag == "--max-states-per-edge" then
      match n.toNat? with
      | some k =>
        parseOptionsFrom (if flag == "--max-states-per-edge" then { o with maxPerEdge := k }
          else { o with maxStates := k }) rest
      | none => .error s!"{flag} expects a non-negative integer, got '{n}'"
    else .error s!"unexpected arguments: {" ".intercalate (flag :: n :: rest)}"
  | args => .error s!"unexpected arguments: {" ".intercalate args}"

def parseOptions (args : List String) : Except String Options := parseOptionsFrom {} args

def usage : String :=
  "Usage: contract-graph-checker <database.sqlite> [--max-states N] [--max-states-per-edge K]\n" ++
  "  --max-states N           stop past N distinct hop states (default " ++
  s!"{defaultMaxStates}; --max-paths N is an alias)\n" ++
  "  --max-states-per-edge K  stop past K distinct composed states on one edge (default " ++
  s!"{defaultMaxPerEdge})"

/-- Main entry point. -/
def main (args : List String) : IO UInt32 := do
  match args with
  | [] =>
    IO.eprintln usage
    IO.eprintln "  (No database path provided)"
    return 2
  | dbPath :: rest =>
    match parseOptions rest with
    | .error msg =>
      IO.eprintln s!"Error: {msg}"
      IO.eprintln usage
      return 2
    | .ok opts =>
      -- Check if the database file exists
      let dbFile : System.FilePath := ⟨dbPath⟩
      let fileExists ← dbFile.pathExists
      if !fileExists then
        IO.eprintln s!"Error: database file not found: {dbPath}"
        return 2
      else do
        -- Read the contract graph from SQLite. A database that cannot be
        -- read or translated is a translation failure (exit code 2), not an
        -- uncaught exception (which the runtime reports as exit code 1).
        let read ← (some <$> readContractGraph dbPath).tryCatch fun e => do
          IO.eprintln s!"Error: {e}"
          pure none
        let some graph := read | return 2
        -- Run the checker pipeline
        let output := runChecker graph opts.maxStates opts.maxPerEdge
        -- Output JSON to stdout
        IO.println (outputToJson output)
        return output.exitCode.toUInt32

end ContractGraph

def main := ContractGraph.main
