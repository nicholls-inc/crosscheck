/-
DRT runner for CrosscheckModel.Ocpp16Composite.

Reads one JSON object on stdin:
  {"cases": [{"connector": 1,
              "times": [<unix seconds>, ...],
              "profiles": [{"connectorId": 0, "stackLevel": 0,
                            "purpose": "chargePointMax" | "txDefault" | "tx",
                            "kind": "absolute" | "daily" | "weekly",
                            "startSchedule": <unix seconds>,
                            "duration": <seconds> | null,
                            "validFrom": <unix seconds> | null,
                            "validTo": <unix seconds> | null,
                            "periods": [[<startPeriod>, <limit in tenths>], ...]}]}]}
Writes {"results": [[<limit in tenths> | null, ...], ...]}, one list per case
and one entry per requested time. Exits 1 on malformed input.
-/
import Lean.Data.Json
import CrosscheckModel.Ocpp16Composite

open Lean
open CrosscheckModel.Ocpp16Composite

def parsePurpose : String → Except String Purpose
  | "chargePointMax" => .ok .chargePointMax
  | "txDefault" => .ok .txDefault
  | "tx" => .ok .tx
  | s => .error s!"unknown purpose {s}"

def parseKind : String → Except String Kind
  | "absolute" => .ok .absolute
  | "daily" => .ok .daily
  | "weekly" => .ok .weekly
  | s => .error s!"unknown kind {s}"

def optNat (j : Json) (key : String) : Except String (Option Nat) :=
  match j.getObjVal? key with
  | .error _ => .ok none
  | .ok Json.null => .ok none
  | .ok v => some <$> v.getNat?

def parsePeriod (j : Json) : Except String Period := do
  let arr ← j.getArr?
  if h : arr.size = 2 then
    return ⟨← arr[0].getNat?, ← arr[1].getNat?⟩
  else
    throw "a period is [startPeriod, limit]"

def parseProfile (j : Json) : Except String Profile := do
  let periods ← (← j.getObjValAs? (Array Json) "periods").toList.mapM parsePeriod
  return {
    connectorId := ← j.getObjValAs? Nat "connectorId"
    stackLevel := ← j.getObjValAs? Nat "stackLevel"
    purpose := ← parsePurpose (← j.getObjValAs? String "purpose")
    kind := ← parseKind (← j.getObjValAs? String "kind")
    startSchedule := ← j.getObjValAs? Nat "startSchedule"
    duration := ← optNat j "duration"
    validFrom := ← optNat j "validFrom"
    validTo := ← optNat j "validTo"
    periods := periods }

def runCase (j : Json) : Except String Json := do
  let c ← j.getObjValAs? Nat "connector"
  let times ← j.getObjValAs? (Array Nat) "times"
  let profiles ← (← j.getObjValAs? (Array Json) "profiles").toList.mapM parseProfile
  return Json.arr (times.map fun t =>
    match composite profiles c t with
    | some l => Json.num l
    | none => Json.null)

def main : IO UInt32 := do
  let input ← (← IO.getStdin).readToEnd
  match Json.parse input >>= (·.getObjValAs? (Array Json) "cases") >>= (·.mapM runCase) with
  | .ok results =>
    IO.println (Json.compress (Json.mkObj [("results", Json.arr results)]))
    return 0
  | .error e =>
    IO.eprintln s!"runner: {e}"
    return 1
