/-
Module: Ocpp16Composite
Source informal spec: formal-verification/specs/ocpp16_composite_informal.md
Source implementation: OCPP 1.6 edition 2 (2017-09-28), sections 3.13.1 to
3.13.3 and 7.8 to 7.14. This model is the spec oracle, so `-- src:` lines
cite spec sections. The system under test is EVerest
lib/everest/ocpp/lib/ocpp/v16/profile.cpp at dd696165.
Sign-off: 2026-10-24
Pipeline step: 4 of 5 (/correspondence-review done). Next: /drt-oracle (3b.6).

Core Lean only, no Mathlib, so it builds with the plain toolchain in
`lean-toolchain` (leanprover/lean4:v4.28.0).

Inventory, from the informal spec:
- Types. `Time` and `Limit` (D3, P5), `Purpose` (3.13.1), `Kind` (P3),
  `Period` (7.14), `Profile` (7.8, 7.13).
- Operations. `applicable` (D1), `validAt` (D2), `scheduleStart` (D3),
  `covers` (D4), `limitAt` (D5), `prevailing` (D6), `purposeLimit`,
  `cpMaxResult` (Q1), `txResult` (Q2), `composite` (Q3), `conforms` (Q4).
- Preconditions P1 to P8 are the predicate `WellFormed`. P3 is also carried
  by `Kind`, which has no Relative constructor.
- Properties. q1, q2, q3 for the postconditions, i1 to i4 for the invariants.
- Ambiguities A1 to A10 are resolved in the informal spec, so no
  `TODO(spec ambiguity)` markers remain.
-/

namespace CrosscheckModel.Ocpp16Composite

-- == Types =================================================================

/-- UTC seconds since the Unix epoch, counted in calendar days of 86400
seconds, so leap seconds are not counted (D3). -/
abbrev Time := Nat

/-- A limit in tenths of the charging rate unit. OCPP 1.6 limits accept at
most one digit fraction (7.14), so tenths are exact. -/
abbrev Limit := Nat

inductive Purpose where
  | chargePointMax
  | txDefault
  | tx
  deriving Repr, DecidableEq

/-- Profile kinds in scope (P3). Relative is out of scope and cannot be
represented. -/
inductive Kind where
  | absolute
  | daily
  | weekly
  deriving Repr, DecidableEq

structure Period where
  startPeriod : Nat
  limit : Limit
  deriving Repr, DecidableEq

structure Profile where
  connectorId : Nat
  stackLevel : Nat
  purpose : Purpose
  kind : Kind
  startSchedule : Time
  duration : Option Nat
  validFrom : Option Time
  validTo : Option Time
  periods : List Period
  deriving Repr, DecidableEq

-- == Definitions ===========================================================

-- src: OCPP 1.6 ed.2 §3.13.1 (purposes and connector ids)
/-- D1. ChargePointMaxProfile on connector 0, TxDefaultProfile on 0 or c,
TxProfile on c. -/
def applicable (c : Nat) (p : Profile) : Bool :=
  match p.purpose with
  | .chargePointMax => p.connectorId == 0
  | .txDefault => p.connectorId == 0 || p.connectorId == c
  | .tx => p.connectorId == c

-- src: OCPP 1.6 ed.2 §7.8 (validFrom, validTo), resolution A4
/-- D2. validFrom ≤ t and t < validTo, each only if present. -/
def validAt (p : Profile) (t : Time) : Bool :=
  (match p.validFrom with
   | some f => decide (f ≤ t)
   | none => true) &&
  (match p.validTo with
   | some v => decide (t < v)
   | none => true)

-- src: OCPP 1.6 ed.2 RecurrencyKindType (Daily, Weekly)
/-- The recurrence period in seconds, or none for an Absolute profile. -/
def recurrence : Kind → Option Nat
  | .absolute => none
  | .daily => some 86400
  | .weekly => some 604800

-- src: OCPP 1.6 ed.2 ChargingProfileKindType and RecurrencyKindType,
-- resolutions A5 and A6
/-- D3. For an Absolute profile, startSchedule. For a Recurring profile, the
latest restart at or before t, and none before startSchedule. -/
def scheduleStart (p : Profile) (t : Time) : Option Time :=
  match recurrence p.kind with
  | none => some p.startSchedule
  | some r =>
    if t < p.startSchedule then none
    else some (p.startSchedule + (t - p.startSchedule) / r * r)

-- src: OCPP 1.6 ed.2 §7.13 (duration)
/-- D4. A schedule start s exists at t, s ≤ t, and t is before s + duration. -/
def covers (p : Profile) (t : Time) : Bool :=
  match scheduleStart p t with
  | none => false
  | some s =>
    decide (s ≤ t) &&
    match p.duration with
    | none => true
    | some d => decide (t < s + d)

-- src: OCPP 1.6 ed.2 §7.14 (startPeriod defines the stop of the previous period)
/-- D5. The limit of the last period with startPeriod ≤ t − start, when the
schedule covers t. -/
def limitAt (p : Profile) (t : Time) : Option Limit :=
  if covers p t then
    match scheduleStart p t with
    | none => none
    | some s => ((p.periods.filter (·.startPeriod ≤ t - s)).getLast?).map (·.limit)
  else none

/-- The candidates for prevailing: applicable, valid and covering at t. -/
def candidates (ps : List Profile) (c : Nat) (purpose : Purpose) (t : Time) :
    List Profile :=
  ps.filter fun p => p.purpose == purpose && applicable c p && validAt p t && covers p t

/-- The first profile with the highest stackLevel. P6 rules out ties among
candidates, so which tied profile is kept never matters in scope. -/
def pickHighest (ps : List Profile) : Option Profile :=
  ps.foldl
    (fun best p =>
      match best with
      | none => some p
      | some b => if b.stackLevel < p.stackLevel then some p else some b)
    none

-- src: OCPP 1.6 ed.2 §3.13.2 (prevailing profile), resolution A1
/-- D6. The applicable, valid, covering profile of a purpose with the
highest stackLevel. -/
def prevailing (ps : List Profile) (c : Nat) (purpose : Purpose) (t : Time) :
    Option Profile :=
  pickHighest (candidates ps c purpose t)

/-- The limit of the prevailing profile of a purpose at t. -/
def purposeLimit (ps : List Profile) (c : Nat) (purpose : Purpose) (t : Time) :
    Option Limit :=
  (prevailing ps c purpose t).bind (limitAt · t)

-- src: OCPP 1.6 ed.2 §3.13.1 and §3.13.3
/-- Q1. -/
def cpMaxResult (ps : List Profile) (c : Nat) (t : Time) : Option Limit :=
  purposeLimit ps c .chargePointMax t

-- src: OCPP 1.6 ed.2 §3.13.1 (TxProfile overrules TxDefaultProfile), resolution A2
/-- Q2. -/
def txResult (ps : List Profile) (c : Nat) (t : Time) : Option Limit :=
  match prevailing ps c .tx t with
  | some p => limitAt p t
  | none => purposeLimit ps c .txDefault t

/-- The minimum of two optional limits, where none means no limit. -/
def minOpt : Option Limit → Option Limit → Option Limit
  | some m, some x => some (min m x)
  | some m, none => some m
  | none, some x => some x
  | none, none => none

-- src: OCPP 1.6 ed.2 §3.13.3 (minimum for each time interval)
/-- Q3. -/
def composite (ps : List Profile) (c : Nat) (t : Time) : Option Limit :=
  minOpt (cpMaxResult ps c t) (txResult ps c t)

-- src: OCPP 1.6 ed.2 §3.13.3 ("SHALL be less than or equal to"), resolution A10
/-- Q4. An implementation's limit conforms when it is at or below the
composite, and always when the composite is none. -/
def conforms (impl : Limit) (model : Option Limit) : Bool :=
  match model with
  | none => true
  | some m => decide (impl ≤ m)

/-- The periods of a schedule start at 0 and strictly increase (P4). -/
def periodsWellFormed : List Period → Prop
  | [] => False
  | p :: rest => p.startPeriod = 0 ∧ (p :: rest).Pairwise (·.startPeriod < ·.startPeriod)

/-- P1 to P8. Units (P5) are not in the model: a case uses one unit. -/
def WellFormed (ps : List Profile) (c : Nat) : Prop :=
  1 ≤ c ∧
  (∀ p ∈ ps, (p.purpose = .chargePointMax → p.connectorId = 0) ∧
             (p.purpose = .tx → 0 < p.connectorId) ∧
             periodsWellFormed p.periods ∧
             (∀ d, p.duration = some d → 0 < d)) ∧
  ps.Pairwise (fun p q => applicable c p → applicable c q →
     p.purpose = q.purpose → p.connectorId = q.connectorId →
     p.stackLevel ≠ q.stackLevel) ∧
  ¬ (∃ p ∈ ps, ∃ q ∈ ps, p.purpose = .txDefault ∧ q.purpose = .txDefault ∧
       p.connectorId = 0 ∧ q.connectorId = c)

-- == Properties ============================================================
-- Q1 to Q3 restate their definitions and hold by unfolding. I1 to I3 are proved; I4's proof is open.
/-- Q1. The ChargePointMaxProfile result is the prevailing
ChargePointMaxProfile's limit. -/
theorem q1 (ps : List Profile) (c : Nat) (t : Time) :
    cpMaxResult ps c t = purposeLimit ps c .chargePointMax t := by
  rfl

/-- Q2. The prevailing TxProfile's limit if one prevails, otherwise the
prevailing TxDefaultProfile's limit. -/
theorem q2 (ps : List Profile) (c : Nat) (t : Time) :
    txResult ps c t =
      match prevailing ps c .tx t with
      | some p => limitAt p t
      | none => purposeLimit ps c .txDefault t := by
  rfl

/-- Q3. The composite is the minimum of the Q1 and Q2 results that exist. -/
theorem q3 (ps : List Profile) (c : Nat) (t : Time) :
    composite ps c t =
      match cpMaxResult ps c t, txResult ps c t with
      | some m, some x => some (min m x)
      | some m, none => some m
      | none, some x => some x
      | none, none => none := by
  unfold composite
  cases cpMaxResult ps c t <;> cases txResult ps c t <;> rfl

/-- I1. The composite never exceeds the ChargePointMaxProfile result. -/
theorem i1 (ps : List Profile) (c : Nat) (t : Time) (m : Limit)
    (h : cpMaxResult ps c t = some m) :
    ∃ x, composite ps c t = some x ∧ x ≤ m := by
  unfold composite
  rw [h]
  cases txResult ps c t with
  | none => exact ⟨m, rfl, Nat.le_refl m⟩
  | some x => exact ⟨min m x, rfl, Nat.min_le_left m x⟩

/-- Candidates of a purpose other than TxDefaultProfile depend only on the
profiles that are not TxDefaultProfiles. -/
theorem candidates_eq_of_same_nonDefault (ps ps' : List Profile) (c : Nat)
    (purpose : Purpose) (t : Time) (hP : purpose ≠ .txDefault)
    (hSame : ps.filter (·.purpose != .txDefault) = ps'.filter (·.purpose != .txDefault)) :
    candidates ps c purpose t = candidates ps' c purpose t := by
  have key : ∀ l : List Profile,
      l.filter (fun p => p.purpose == purpose && applicable c p && validAt p t && covers p t) =
      (l.filter (·.purpose != .txDefault)).filter
        (fun p => p.purpose == purpose && applicable c p && validAt p t && covers p t) := by
    intro l
    rw [List.filter_filter]
    apply List.filter_congr
    intro x _
    by_cases hx : x.purpose = purpose
    · subst hx
      simp [hP]
    · simp [hx]
  unfold candidates
  rw [key ps, key ps', hSame]

/-- I2. While a TxProfile prevails, TxDefaultProfiles do not affect the
composite. -/
theorem i2 (ps ps' : List Profile) (c : Nat) (t : Time)
    (hTx : (prevailing ps c .tx t).isSome)
    (hSame : ps.filter (·.purpose != .txDefault) = ps'.filter (·.purpose != .txDefault)) :
    composite ps c t = composite ps' c t := by
  have hpc : prevailing ps c .chargePointMax t = prevailing ps' c .chargePointMax t := by
    unfold prevailing
    rw [candidates_eq_of_same_nonDefault ps ps' c .chargePointMax t (by decide) hSame]
  have hpt : prevailing ps c .tx t = prevailing ps' c .tx t := by
    unfold prevailing
    rw [candidates_eq_of_same_nonDefault ps ps' c .tx t (by decide) hSame]
  obtain ⟨p, hp⟩ := Option.isSome_iff_exists.mp hTx
  have hp' : prevailing ps' c .tx t = some p := hpt ▸ hp
  unfold composite cpMaxResult purposeLimit txResult
  rw [hpc, hp, hp']

/-- I3. The composite never exceeds the Q2 result. -/
theorem i3 (ps : List Profile) (c : Nat) (t : Time) (x : Limit)
    (h : txResult ps c t = some x) :
    ∃ y, composite ps c t = some y ∧ y ≤ x := by
  unfold composite
  rw [h]
  cases cpMaxResult ps c t with
  | none => exact ⟨x, rfl, Nat.le_refl x⟩
  | some m => exact ⟨min m x, rfl, Nat.min_le_right m x⟩

/-- I4. A profile below the prevailing stack level of its purpose does not
affect the composite. -/
theorem i4 (ps : List Profile) (c : Nat) (t : Time) (q p : Profile)
    (hp : prevailing ps c q.purpose t = some p)
    (hlow : q.stackLevel < p.stackLevel) :
    composite (q :: ps) c t = composite ps c t := by
  sorry

-- == Smoke tests: worked examples from the informal spec ===================

/-- 2026-01-01T00:00:00Z. -/
def jan1 : Time := 1767225600
def hour : Nat := 3600

/-- Worked example 1, the section 3.13.7 TxDefaultProfile, in tenths of W. -/
def ex1 : List Profile :=
  [{ connectorId := 1, stackLevel := 0, purpose := .txDefault, kind := .daily,
     startSchedule := 1356998400, duration := some 86400, validFrom := none,
     validTo := none,
     periods := [⟨0, 110000⟩, ⟨28800, 60000⟩, ⟨72000, 110000⟩] }]

/-- Worked example 2: ChargePointMaxProfile 16 A and TxProfile 32 A. -/
def ex2 : List Profile :=
  [{ connectorId := 0, stackLevel := 0, purpose := .chargePointMax, kind := .absolute,
     startSchedule := jan1, duration := none, validFrom := none, validTo := none,
     periods := [⟨0, 160⟩] },
   { connectorId := 1, stackLevel := 0, purpose := .tx, kind := .absolute,
     startSchedule := jan1, duration := none, validFrom := none, validTo := none,
     periods := [⟨0, 320⟩] }]

/-- Worked example 3: a higher stack level for one hour only. -/
def ex3 : List Profile :=
  [{ connectorId := 1, stackLevel := 0, purpose := .txDefault, kind := .absolute,
     startSchedule := jan1, duration := none, validFrom := none, validTo := none,
     periods := [⟨0, 100⟩] },
   { connectorId := 1, stackLevel := 1, purpose := .txDefault, kind := .absolute,
     startSchedule := jan1 + 10 * hour, duration := some hour, validFrom := none,
     validTo := none, periods := [⟨0, 200⟩] }]

/-- Worked example 4: a 12-hour TxProfile over a TxDefaultProfile. -/
def ex4 : List Profile :=
  [{ connectorId := 1, stackLevel := 0, purpose := .tx, kind := .absolute,
     startSchedule := jan1, duration := some 43200, validFrom := none, validTo := none,
     periods := [⟨0, 60⟩] },
   { connectorId := 0, stackLevel := 0, purpose := .txDefault, kind := .absolute,
     startSchedule := jan1, duration := none, validFrom := none, validTo := none,
     periods := [⟨0, 320⟩] }]

/-- Worked example 5: a ChargePointMaxProfile valid until noon. -/
def ex5 : List Profile :=
  [{ connectorId := 0, stackLevel := 0, purpose := .chargePointMax, kind := .absolute,
     startSchedule := jan1, duration := none, validFrom := none,
     validTo := some (jan1 + 12 * hour), periods := [⟨0, 80⟩] },
   { connectorId := 1, stackLevel := 0, purpose := .txDefault, kind := .absolute,
     startSchedule := jan1, duration := none, validFrom := none, validTo := none,
     periods := [⟨0, 160⟩] }]

#guard (composite ex1 1 jan1, composite ex1 1 (jan1 + 28799), composite ex1 1 (jan1 + 28800),
        composite ex1 1 (jan1 + 71999), composite ex1 1 (jan1 + 72000)) =
       (some 110000, some 110000, some 60000, some 60000, some 110000)
#guard (composite ex2 1 jan1, composite ex2 1 (jan1 + 20 * hour)) = (some 160, some 160)
#guard (composite ex3 1 (jan1 + 9 * hour), composite ex3 1 (jan1 + 10 * hour),
        composite ex3 1 (jan1 + 11 * hour)) = (some 100, some 200, some 100)
#guard (composite ex4 1 (jan1 + 11 * hour), composite ex4 1 (jan1 + 12 * hour)) =
       (some 60, some 320)
#guard (composite ex5 1 (jan1 + 11 * hour), composite ex5 1 (jan1 + 12 * hour)) =
       (some 80, some 160)

end CrosscheckModel.Ocpp16Composite
