-- Main.lean

import ContractGraph.Types
import ContractGraph.BehaviorModel
import ContractGraph.DependentExpr
import ContractGraph.Translation
import ContractGraph.Checker
import ContractGraph.Composition
import ContractGraph.Diagnostics

namespace ContractGraph

/-- Escape a string for JSON output. -/
private def jsonEscape (s : String) : String :=
  s.replace "\\" "\\\\" |>.replace "\"" "\\\"" |>.replace "\n" "\\n"

/-- Format a SourceLocation as JSON. -/
private def sourceLocationToJson (loc : SourceLocation) : String :=
  s!"\{\"file\": \"{jsonEscape loc.file}\", \"line\": {loc.line}, \"name\": \"{jsonEscape loc.name}\"}"

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

/-- Collect all inconsistencies from path check results. -/
def collectResults (pathResults : List (List Edge × List CheckResult))
    : List ResultEntry :=
  pathResults.flatMap fun (path, results) =>
    let pathNames := match path with
      | [] => []
      | edges => (edges.map (·.source.name)) ++
        match edges.getLast? with
        | some last => [last.target.name]
        | none => []
    let verLevel := pathVerificationLevel path
    results.filterMap fun r =>
      match r with
      | .consistent => none
      | .inconsistent diag =>
        some (buildResultEntry { diag with path := pathNames } pathNames verLevel)

/-- Two entries report the same inconsistency when everything except the
    path matches: the same source constraint against the same target
    constraint on the same hop (`target.name` is the hop target; `hop` also
    names the hop source). -/
def sameFinding (a b : ResultEntry) : Bool :=
  a.severity == b.severity &&
  a.source.file == b.source.file && a.source.line == b.source.line &&
  a.target.file == b.target.file && a.target.line == b.target.line &&
  a.target.name == b.target.name && a.hop == b.hop &&
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

/-- Count total contracts across all nodes. -/
def countContracts (nodes : List Node) : Nat :=
  nodes.foldl (fun acc n => acc + n.preconditions.length + n.postconditions.length) 0

/-- Run the full checking pipeline on a contract graph. -/
def runChecker (graph : ContractGraph) : CheckOutput :=
  let pathResults := checkAllPaths graph
  let results := dedupeResults (collectResults pathResults)
  let hasErrors := results.any (·.severity == "error")
  { summary := {
      contractsChecked := countContracts graph.nodes
      edgesChecked := graph.edges.length
      pathsChecked := pathResults.length
    }
    results := results
    exitCode := if hasErrors then 1 else 0
  }

/-! ## Soundness of the executable's verdict

Exit code 0 means no reported entry has severity "error". Every error
`CheckResult` of every checked path yields such an entry in `collectResults`,
and deduplication never removes the last entry of a given severity. Hence exit
code 0 implies no path has an error result, and `checkPath_sound_noErrors`
gives `stepwiseSound` for every enumerated path. -/

/-- Exit code 0 iff no reported entry has severity "error". -/
theorem runChecker_exitCode_eq_zero_iff (g : ContractGraph) :
    (runChecker g).exitCode = 0 ↔ ∀ e ∈ (runChecker g).results, e.severity ≠ "error" := by
  simp only [runChecker]
  split
  · rename_i hany
    obtain ⟨e, he, hs⟩ := List.any_eq_true.mp hany
    simp only [beq_iff_eq] at hs
    exact ⟨fun h => absurd h (by decide), fun h => absurd hs (h e he)⟩
  · rename_i hany
    refine ⟨fun _ e he hs => hany (List.any_eq_true.mpr ⟨e, he, by simp [hs]⟩), fun _ => rfl⟩

/-- Entries that are the same finding have the same severity. -/
theorem sameFinding_severity {a b : ResultEntry} (h : sameFinding a b = true) :
    a.severity = b.severity := by
  unfold sameFinding at h
  simp only [Bool.and_eq_true, beq_iff_eq] at h
  exact h.1.1.1.1.1.1.1.1.1

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

/-- Deduplication keeps at least one entry of every severity in its input. -/
theorem dedupeResults_keeps (rs : List ResultEntry) (e : ResultEntry) (he : e ∈ rs) :
    ∃ y ∈ dedupeResults rs, y.severity = e.severity := by
  unfold dedupeResults
  suffices ∀ acc, ∃ y ∈ rs.foldl dedupeStep acc, y.severity = e.severity from this []
  induction rs with
  | nil => cases he
  | cons hd tl ih =>
    intro acc
    rcases List.mem_cons.mp he with rfl | htl
    · exact foldl_dedupeStep_keeps tl _ _ (dedupeStep_adds acc e _ rfl)
    · exact ih htl _

/-- Every error result of a checked path is reported with severity "error". -/
theorem collectResults_error (pathResults : List (List Edge × List CheckResult))
    (p : List Edge) (rs : List CheckResult) (r : CheckResult)
    (hmem : (p, rs) ∈ pathResults) (hr : r ∈ rs) (herr : r.isError = true) :
    ∃ e ∈ collectResults pathResults, e.severity = "error" := by
  cases r with
  | consistent => cases herr
  | inconsistent d =>
    have hsev : d.severity = .error := by
      cases hd : d.severity
      · rfl
      · simp [CheckResult.isError, hd] at herr
    refine ⟨_, List.mem_flatMap.mpr ⟨(p, rs), hmem, List.mem_filterMap.mpr ⟨_, hr, rfl⟩⟩, ?_⟩
    simp [buildResultEntry, hsev, toString]

/-- END-TO-END SOUNDNESS. If the checker exits with code 0 (warnings allowed),
    every enumerated path is stepwise sound: at every hop, the (composed)
    source guarantees imply the target's requirements. -/
theorem runChecker_sound (g : ContractGraph) (h : (runChecker g).exitCode = 0) :
    ∀ p ∈ enumeratePaths g, p ≠ [] → stepwiseSound p := by
  intro p hp hne
  apply checkPath_sound_noErrors p hne
  intro r hr
  cases herr : r.isError with
  | false => rfl
  | true =>
    exfalso
    have hmem : (p, checkPath p) ∈ checkAllPaths g := List.mem_map.mpr ⟨p, hp, rfl⟩
    obtain ⟨e, he, hs⟩ := collectResults_error _ p _ r hmem hr herr
    obtain ⟨y, hy, hys⟩ := dedupeResults_keeps _ e he
    exact (runChecker_exitCode_eq_zero_iff g).mp h y hy (hys.trans hs)

/-- END-TO-END SOUNDNESS over all data paths. If the checker exits with code 0,
    every checked data path of the graph (`IsDataPath`: simple, non-`calls`
    edges, function node to model node) is stepwise sound — not only the ones
    the enumeration happened to produce (`enumeratePaths_complete`). -/
theorem runChecker_sound_all (g : ContractGraph) (h : (runChecker g).exitCode = 0) :
    ∀ p, IsDataPath g p → stepwiseSound p :=
  fun p hp => runChecker_sound g h p (enumeratePaths_complete g p hp) hp.1

/-- Main entry point. -/
def main (args : List String) : IO UInt32 := do
  if args.isEmpty then
    IO.eprintln "Usage: contract-graph-checker <database.sqlite>"
    IO.eprintln "  (No database path provided)"
    return 2
  else do
    let dbPath := args.head!
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
      let output := runChecker graph
      -- Output JSON to stdout
      IO.println (outputToJson output)
      return output.exitCode.toUInt32

end ContractGraph

def main := ContractGraph.main
