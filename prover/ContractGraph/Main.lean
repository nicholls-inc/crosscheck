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
    constraint on the same target node. -/
def sameFinding (a b : ResultEntry) : Bool :=
  a.severity == b.severity &&
  a.source.file == b.source.file && a.source.line == b.source.line &&
  a.target.file == b.target.file && a.target.line == b.target.line &&
  a.target.name == b.target.name &&
  a.sourceGuarantee == b.sourceGuarantee &&
  a.targetRequirement == b.targetRequirement &&
  a.suggestion == b.suggestion

/-- Report each inconsistency once. When several paths reach the same finding
    (e.g. a caller's `calls` edge prefixed to the path from the function that
    owns the constraint), keep the shortest path, which starts at the owner. -/
def dedupeResults (results : List ResultEntry) : List ResultEntry :=
  results.foldl (fun acc r =>
    if acc.any (sameFinding · r) then
      acc.map fun e => if sameFinding e r && r.path.length < e.path.length then r else e
    else
      acc ++ [r]) []

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
