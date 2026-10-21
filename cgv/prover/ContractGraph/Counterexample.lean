-- Counterexample.lean

import ContractGraph.Types
import ContractGraph.Checker

namespace ContractGraph

/-- A value that an error's source constraint admits and its target
    constraint rejects (rules WT-1 to WT-7 of
    `intent/2026-10-07-error-witness-spec.md`). One shape per constraint kind
    it answers. -/
inductive Counterexample where
  /-- A string of `n` characters, `"x" * n` (`length`). -/
  | str (n : Nat)
  /-- The number 10^-n, with exactly `n` fractional digits (`precision`). -/
  | fraction (n : Nat)
  /-- The number m / 10^scale (`range`, `range_min`). -/
  | scaled (m : Int) (scale : Nat)
  /-- A choice value (`choices`), with the Python type of the target
      field's values when the extractor knows it (`"int"` or `"str"`), which
      decides how it is written (WT-6). -/
  | choice (c : String) (ty : Option String)
  /-- `None` (`nullability`). -/
  | none
  deriving Repr, BEq, DecidableEq

private def atMost (x : Int) : Option Int → Bool
  | some b => decide (x ≤ b)
  | none => true

private def atLeast (x : Int) : Option Int → Bool
  | some b => decide (b ≤ x)
  | none => true

/-- Whether constraint `c` admits the value, with the comparison that
    `constraintImplies` uses for the kind (WT-10). A value of the shape of one
    kind is not admitted by a constraint of another. A missing bound admits
    every value of the kind. -/
def Counterexample.admits (v : Counterexample) (c : Constraint) : Bool :=
  match v with
  | .str n => decide (c.kind = .length) && atMost n c.staticBound
  | .fraction n => decide (c.kind = .precision) && atMost n c.staticBound
  | .scaled m _ =>
    (decide (c.kind = .range) && atMost m c.staticBound) ||
    (decide (c.kind = .rangeMin) && atLeast m c.staticBound)
  | .choice x _ =>
    decide (c.kind = .choices) &&
      (match c.choicesList with
       | some cs => cs.contains x
       | _ => true)
  | .none => decide (c.kind = .nullability) && c.staticBound != some 0

/-- The value WT-2 to WT-7 give for a source constraint `s` against a target
    `t`, before it is checked. `none` for a `type` constraint and for the three
    cases WT-1 leaves out: a length bound below 0, a precision bound below 1,
    and range bounds of different scales. -/
def counterexampleCandidate (s t : Constraint) : Option Counterexample :=
  match s.kind, s.staticBound with
  | .length, some b => if 0 ≤ b then some (.str b.toNat) else none
  | .precision, some b => if 1 ≤ b then some (.fraction b.toNat) else none
  | .range, some b | .rangeMin, some b =>
    if s.scale = t.scale then some (.scaled b s.scale) else none
  | .nullability, _ => some .none
  | .choices, _ =>
    match s.choicesList, t.choicesList with
    | some sc, some tc => (sc.find? (fun x => !tc.contains x)).map (.choice · t.typeName)
    | _, _ => none
  | _, _ => none

/-- The counterexample of a source constraint `s` against a target `t`: the
    candidate, kept only when `s` admits it and `t` rejects it, so every value
    this returns is one (`counterexampleOf_sound`). -/
def counterexampleOf (s t : Constraint) : Option Counterexample :=
  if s.kind = t.kind then
    match counterexampleCandidate s t with
    | some v => if v.admits s && !v.admits t then some v else none
    | none => none
  else none

/-- The counterexample of a diagnostic: of its two constraints for an error,
    none for a warning (WT-1). -/
def DiagnosticInfo.counterexample (d : DiagnosticInfo) : Option Counterexample :=
  match d.severity with
  | .error => counterexampleOf d.sourceConstraint d.targetConstraint
  | .warning => none

theorem counterexampleOf_sound {s t : Constraint} {v : Counterexample}
    (h : counterexampleOf s t = some v) : v.admits s = true ∧ v.admits t = false := by
  unfold counterexampleOf at h
  split at h
  · split at h
    · split at h
      · cases h
        rename_i hv
        simpa using hv
      · cases h
    · cases h
  · cases h

theorem DiagnosticInfo.counterexample_sound {d : DiagnosticInfo} {v : Counterexample}
    (h : d.counterexample = some v) :
    v.admits d.sourceConstraint = true ∧ v.admits d.targetConstraint = false := by
  unfold DiagnosticInfo.counterexample at h
  split at h
  · exact counterexampleOf_sound h
  · cases h

/-- WT-10. Every error that `checkConstraintPair` reports for two constraints
    of kind `length`, `precision`, `range`, `range_min`, `choices` or
    `nullability` has a counterexample, which the source admits and the target
    rejects, except the three cases of WT-1, which the hypotheses exclude. -/
theorem checkConstraintPair_counterexample (s t : Constraint) (d : DiagnosticInfo)
    (h : checkConstraintPair s t = .inconsistent d)
    (hType : s.kind ≠ .type)
    (hLength : s.kind = .length → ∀ b, s.staticBound = some b → 0 ≤ b)
    (hPrecision : s.kind = .precision → ∀ b, s.staticBound = some b → 1 ≤ b)
    (hScale : s.kind = .range ∨ s.kind = .rangeMin → s.scale = t.scale) :
    d.sourceConstraint = s ∧ d.targetConstraint = t ∧
      ∃ v, d.counterexample = some v ∧ v.admits s = true ∧ v.admits t = false := by
  unfold checkConstraintPair at h
  by_cases hk : s.kind = t.kind
  · have hne : ¬ (s.kind != t.kind) = true := by
      rw [hk]; cases t.kind <;> decide
    rw [if_neg hne] at h
    have hd : ∀ (d' : DiagnosticInfo), d'.severity = .error → d'.sourceConstraint = s →
        d'.targetConstraint = t → d = d' →
        (∃ v, counterexampleCandidate s t = some v ∧ v.admits s = true ∧ v.admits t = false) →
        d.sourceConstraint = s ∧ d.targetConstraint = t ∧
          ∃ v, d.counterexample = some v ∧ v.admits s = true ∧ v.admits t = false := by
      intro d' hsev hs ht hdd ⟨v, hc, ha, hr⟩
      subst hdd
      refine ⟨hs, ht, v, ?_, ha, hr⟩
      simp [DiagnosticInfo.counterexample, hsev, hs, ht, counterexampleOf, hk, hc, ha, hr]
    revert h
    cases hsk : s.kind <;> simp only [] <;> intro h
    · -- precision
      cases hsb : s.staticBound <;> cases htb : t.staticBound <;> simp [hsb, htb] at h
      rename_i sg tr
      unfold checkStaticBounds at h
      split at h
      · cases h
      · rename_i hlt
        cases h
        apply hd _ rfl rfl rfl rfl
        have h1 := hPrecision hsk sg hsb
        refine ⟨.fraction sg.toNat, ?_, ?_, ?_⟩
        · simp [counterexampleCandidate, hsk, hsb, h1]
        · simp [Counterexample.admits, hsk, hsb, atMost]; omega
        · simp [Counterexample.admits, ← hk, hsk, htb, atMost]; omega
    · -- nullability
      cases hsb : s.staticBound <;> cases htb : t.staticBound <;> simp [hsb, htb] at h
      rename_i sn tn
      unfold checkNullability at h
      split at h
      · rename_i hc
        cases h
        apply hd _ rfl rfl rfl rfl
        simp at hc
        refine ⟨.none, ?_, ?_, ?_⟩
        · simp [counterexampleCandidate, hsk]
        · simp [Counterexample.admits, hsk, hsb, hc.1]
        · simp [Counterexample.admits, ← hk, hsk, htb, hc.2]
      · cases h
    · exact absurd hsk hType
    · -- range
      cases hsb : s.staticBound <;> cases htb : t.staticBound <;> simp [hsb, htb] at h
      rename_i sg tr
      unfold checkStaticBounds at h
      split at h
      · cases h
      · cases h
        apply hd _ rfl rfl rfl rfl
        refine ⟨.scaled sg s.scale, ?_, ?_, ?_⟩
        · simp [counterexampleCandidate, hsk, hsb, hScale (.inl hsk)]
        · simp [Counterexample.admits, hsk, hsb, atMost]
        · simp [Counterexample.admits, ← hk, hsk, htb, atMost, atLeast]; omega
    · -- length
      cases hsb : s.staticBound <;> cases htb : t.staticBound <;> simp [hsb, htb] at h
      rename_i sg tr
      unfold checkStaticBounds at h
      split at h
      · cases h
      · cases h
        apply hd _ rfl rfl rfl rfl
        have h0 := hLength hsk sg hsb
        refine ⟨.str sg.toNat, ?_, ?_, ?_⟩
        · simp [counterexampleCandidate, hsk, hsb, h0]
        · simp [Counterexample.admits, hsk, hsb, atMost]; omega
        · simp [Counterexample.admits, ← hk, hsk, htb, atMost]; omega
    · -- choices
      cases hsc : s.choicesList <;> cases htc : t.choicesList <;> simp [hsc, htc] at h
      rename_i sc tc
      unfold checkChoicesSubset at h
      split at h
      · cases h
      · rename_i hall
        cases h
        apply hd _ rfl rfl rfl rfl
        cases hf : sc.find? (fun x => !tc.contains x) with
        | none =>
          exfalso
          apply hall
          rw [List.find?_eq_none] at hf
          simpa using hf
        | some x =>
          have hx := List.mem_of_find?_eq_some hf
          have hnx := List.find?_some hf
          refine ⟨.choice x t.typeName, ?_, ?_, ?_⟩
          · simp [counterexampleCandidate, hsk, hsc, htc]
            simpa using hf
          · simp [Counterexample.admits, hsk, hsc, hx]
          · simp at hnx
            simp [Counterexample.admits, ← hk, hsk, htc, hnx]
    · -- range_min
      cases hsb : s.staticBound <;> cases htb : t.staticBound <;> simp [hsb, htb] at h
      rename_i sg tr
      unfold checkLowerBounds at h
      split at h
      · cases h
      · cases h
        apply hd _ rfl rfl rfl rfl
        refine ⟨.scaled sg s.scale, ?_, ?_, ?_⟩
        · simp [counterexampleCandidate, hsk, hsb, hScale (.inr hsk)]
        · simp [Counterexample.admits, hsk, hsb, atLeast]
        · simp [Counterexample.admits, ← hk, hsk, htb, atMost, atLeast]; omega
  · have hne : (s.kind != t.kind) = true := by
      revert hk; cases s.kind <;> cases t.kind <;> decide
    rw [if_pos hne] at h
    cases h

/-! ### Python text (WT-2 to WT-7)

Not covered by the theorem: `ContractGraphTest/Counterexample.lean` pins it
for each kind. -/

/-- `n` in lowercase hexadecimal, zero-padded to `width` digits. -/
private def hexPadded (n width : Nat) : String :=
  let ds := Nat.toDigits 16 n
  String.ofList (List.replicate (width - ds.length) '0' ++ ds)

/-- One character of a double-quoted Python string literal (WT-6). -/
def pythonEscapeChar (c : Char) : String :=
  if c = '\\' then "\\\\"
  else if c = '"' then "\\\""
  else if c.toNat < 0x20 || 0x7f ≤ c.toNat then
    if c.toNat ≤ 0xff then "\\x" ++ hexPadded c.toNat 2
    else if c.toNat ≤ 0xffff then "\\u" ++ hexPadded c.toNat 4
    else "\\U" ++ hexPadded c.toNat 8
  else c.toString

/-- A double-quoted Python string literal whose value is `s` (WT-6). -/
def pythonStringLiteral (s : String) : String :=
  "\"" ++ String.join (s.toList.map pythonEscapeChar) ++ "\""

/-- The value as Python source text (WT-2 to WT-7). -/
def Counterexample.toPython : Counterexample → String
  | .str n => s!"\"x\" * {n}"
  | .fraction n => "Decimal(\"0." ++ "".pushn '0' (n - 1) ++ "1\")"
  | .scaled m scale =>
    if m.natAbs % 10 ^ scale = 0 then formatScaled m scale
    else s!"Decimal(\"{formatScaled m scale}\")"
  | .choice c ty =>
    match ty, c.toInt? with
    | some "int", some n => if toString n = c then toString n else pythonStringLiteral c
    | _, _ => pythonStringLiteral c
  | .none => "None"

end ContractGraph
