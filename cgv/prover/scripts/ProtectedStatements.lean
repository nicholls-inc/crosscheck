/-
Prints the protected soundness statements of CGV, one block per constant, in a
fixed order. CI compares the output with the committed manifest
`cgv/prover/protected-statements.txt`; any difference fails the build.

  lake build ContractGraph ContractGraph.Main
  lake env lean --run scripts/ProtectedStatements.lean > protected-statements.txt

What is printed:
* every theorem in `protectedTheorems`: its statement (type);
* every definition in `protectedDefinitions`, and every non-theorem
  `ContractGraph.*` constant their types or values reach: its type, plus a
  structural hash of its value. Inductive types list their constructors.

Proofs are never printed, so rewriting a proof leaves the manifest unchanged.
A proof must still hold, though: every protected theorem and definition may
depend only on the axioms in `allowedAxioms`. A `sorry`, or a new axiom that
closes a proof, makes this script fail instead of printing. A declaration added
with the kernel check switched off (`debug.skipKernelTC`) reports no axioms, so
this check does not reach it; see `.claude/rules/protected-surfaces.md`.

The theorems are about the definitions, but the checker binary runs their
compiled code. `@[implemented_by]` and `@[extern]` replace that code without an
axiom, so the script also fails if either sits on a project constant the
compiled code of a protected theorem or definition can run (`checkCompiledCode`).

The list of names mirrors the CGV table in `.claude/rules/protected-surfaces.md`;
keep the two in step.
-/
import Lean
open Lean Meta

def protectedTheorems : List Name := [
  `ContractGraph.checkEdge_sound,
  `ContractGraph.checkEdgeAll_sound,
  `ContractGraph.checkEdgeAll_sound_noErrors,
  `ContractGraph.checkPath_sound,
  `ContractGraph.checkPath_sound_noErrors,
  `ContractGraph.enumeratePaths_complete,
  `ContractGraph.closedStates_checkPath,
  `ContractGraph.runChecker_sound,
  `ContractGraph.runChecker_sound_all,
  `ContractGraph.runChecker_exitCode_eq_zero_iff,
  `ContractGraph.runCheckerPaths_exitCode_eq_zero_iff,
  `ContractGraph.incompleteWith_exitCode
]

def protectedDefinitions : List Name := [
  `ContractGraph.constraintImplies,
  `ContractGraph.IsDataPath,
  `ContractGraph.stepwiseSound
]

/-- Lean's standard axioms. `sorryAx`, and any axiom declared in the project,
are not on this list. -/
def allowedAxioms : List Name := [``propext, ``Classical.choice, ``Quot.sound]

/-- Fails unless `n` depends only on `allowedAxioms`. -/
def checkAxioms (n : Name) : MetaM Unit := do
  let bad := (← collectAxioms n).filter (!allowedAxioms.contains ·)
  unless bad.isEmpty do
    throwError "{n} depends on disallowed axioms {bad.toList}; a protected proof may use only {allowedAxioms}"

/-- Defined in one of CGV's own modules, whatever its namespace: private
helpers and root-namespace instances count. -/
def inProject (env : Environment) (n : Name) : Bool :=
  match env.getModuleIdxFor? n with
  | some i => (`ContractGraph).isPrefixOf env.header.moduleNames[i.toNat]!
  | none => false

/-- Constants the compiled code of `ci` can run. Types are included to stay
conservative. The compiler runs a recursive definition's `_unsafe_rec` helper
in its place. -/
def runtimeDependencies (env : Environment) (ci : ConstantInfo) : Array Name := Id.run do
  let mut ds := ci.type.getUsedConstants
  if let some v := ci.value? (allowOpaque := true) then ds := ds ++ v.getUsedConstants
  if let .inductInfo v := ci then ds := ds ++ v.ctors.toArray
  let unsafeRec := ci.name ++ `_unsafe_rec
  if env.contains unsafeRec then ds := ds.push unsafeRec
  return ds

/-- Every project constant reachable from the protected theorems' statements
and the protected definitions. Theorems are not entered: proofs are erased
from compiled code. Library constants are not entered: they cannot refer back
to project code. -/
partial def runtimeReach (env : Environment) : MetaM (Array Name) := do
  let mut todo := protectedDefinitions
  for t in protectedTheorems do
    let some ci := env.find? t | throwError "protected theorem {t} not found"
    todo := ci.type.getUsedConstants.toList ++ todo
  let mut seen : NameSet := {}
  while !todo.isEmpty do
    match todo with
    | [] => pure ()
    | n :: rest =>
      todo := rest
      if seen.contains n || !inProject env n then continue
      match env.find? n with
      | none | some (.thmInfo _) => continue
      | some ci =>
        seen := seen.insert n
        todo := (runtimeDependencies env ci).toList ++ todo
  return seen.toArray.qsort Name.lt

/-- Fails if the compiled code of a reached constant is not its definition. -/
def checkCompiledCode (env : Environment) : MetaM Unit := do
  let mut bad : Array String := #[]
  for n in ← runtimeReach env do
    if let some impl := Compiler.implementedByAttr.getParam? env n then
      bad := bad.push s!"{n} (implemented_by {impl})"
    if isExtern env n then
      bad := bad.push s!"{n} (extern)"
  unless bad.isEmpty do
    throwError "the compiled code of these constants is not their definition, so the soundness theorems do not cover what the checker binary runs: {bad.toList}"

def inScope (n : Name) : Bool :=
  (`ContractGraph).isPrefixOf n

/-- Constants a definition's meaning depends on: those in its type and value,
and for an inductive type its constructors. Theorems are skipped: their proofs
do not change what a definition means. -/
def dependencies (ci : ConstantInfo) : Array Name :=
  let fromExprs := ci.type.getUsedConstants ++ (ci.value?.map (·.getUsedConstants) |>.getD #[])
  match ci with
  | .inductInfo v => fromExprs ++ v.ctors.toArray
  | _ => fromExprs

/-- Every in-scope, non-theorem constant reachable from `roots`, sorted. -/
partial def reach (env : Environment) (roots : List Name) : Array Name := Id.run do
  let mut seen : NameSet := {}
  let mut todo := roots
  while !todo.isEmpty do
    match todo with
    | [] => pure ()
    | n :: rest =>
      todo := rest
      if seen.contains n then continue
      match env.find? n with
      | none => continue
      | some (.thmInfo _) => continue
      | some ci =>
        seen := seen.insert n
        for d in dependencies ci do
          if inScope d && !seen.contains d then todo := d :: todo
  return seen.toArray.qsort Name.lt

def kindOf : ConstantInfo → String
  | .thmInfo _ => "theorem"
  | .defnInfo _ => "def"
  | .opaqueInfo _ => "opaque"
  | .axiomInfo _ => "axiom"
  | .inductInfo _ => "inductive"
  | .ctorInfo _ => "constructor"
  | .recInfo _ => "recursor"
  | .quotInfo _ => "quot"

def render (env : Environment) : MetaM String := do
  checkCompiledCode env
  let mut out := "-- Generated by cgv/prover/scripts/ProtectedStatements.lean. Do not edit by hand.\n"
  out := out ++ "-- A change here is a change to CGV's guarantee: see .claude/rules/protected-surfaces.md.\n"
  out := out ++ "\n-- Protected theorem statements\n"
  for n in protectedTheorems do
    let some ci := env.find? n | throwError "protected theorem {n} not found"
    unless ci matches .thmInfo _ do throwError "{n} is not a theorem"
    checkAxioms n
    out := out ++ s!"\ntheorem {n} :\n  {← ppExpr ci.type}\n"
  out := out ++ "\n-- Protected definitions and every ContractGraph definition they reach\n"
  for n in protectedDefinitions do
    unless env.contains n do throwError "protected definition {n} not found"
    checkAxioms n
  for n in reach env protectedDefinitions do
    let some ci := env.find? n | unreachable!
    let valueHash := match ci.value? with
      | some v => s!"\n  value hash {hash v}"
      | none => ""
    out := out ++ s!"\n{kindOf ci} {n} :\n  {← ppExpr ci.type}{valueHash}\n"
  return out

def main : IO UInt32 := do
  initSearchPath (← findSysroot)
  let env ← importModules #[{ module := `ContractGraph }, { module := `ContractGraph.Main }] {}
  let opts : Options := Options.empty |>.set `format.width (100 : Nat)
  let ctx : Core.Context := { fileName := "<ProtectedStatements>", fileMap := default, options := opts }
  let (out, _) ← (render env).run' {} |>.toIO ctx { env }
  IO.print out
  return 0
