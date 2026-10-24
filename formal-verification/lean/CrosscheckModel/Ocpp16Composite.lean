/-
Module: Ocpp16Composite
Source informal spec: formal-verification/specs/ocpp16_composite_informal.md
Sign-off: 2026-10-24
Pipeline step: 2 of 5 (/lean-spec). Next: /lean-impl (3b.4).

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

-- == Signatures ============================================================

/-- D1. ChargePointMaxProfile on connector 0, TxDefaultProfile on 0 or c,
TxProfile on c. -/
def applicable (c : Nat) (p : Profile) : Bool := sorry

/-- D2. validFrom ≤ t and t < validTo, each only if present. -/
def validAt (p : Profile) (t : Time) : Bool := sorry

/-- D3. The schedule start in force at t, or none before startSchedule. -/
def scheduleStart (p : Profile) (t : Time) : Option Time := sorry

/-- D4. A schedule start exists at t and t is before start + duration. -/
def covers (p : Profile) (t : Time) : Bool := sorry

/-- D5. The limit of the last period with startPeriod ≤ t − start. -/
def limitAt (p : Profile) (t : Time) : Option Limit := sorry

/-- D6. The applicable, valid, covering profile of a purpose with the
highest stackLevel. -/
def prevailing (ps : List Profile) (c : Nat) (purpose : Purpose) (t : Time) :
    Option Profile := sorry

/-- The limit of the prevailing profile of a purpose at t. -/
def purposeLimit (ps : List Profile) (c : Nat) (purpose : Purpose) (t : Time) :
    Option Limit := sorry

/-- Q1. -/
def cpMaxResult (ps : List Profile) (c : Nat) (t : Time) : Option Limit := sorry

/-- Q2. -/
def txResult (ps : List Profile) (c : Nat) (t : Time) : Option Limit := sorry

/-- Q3. -/
def composite (ps : List Profile) (c : Nat) (t : Time) : Option Limit := sorry

/-- Q4. An implementation's limit conforms when it is at or below the
composite, and always when the composite is none. -/
def conforms (impl : Limit) (model : Option Limit) : Bool := sorry

/-- P1 to P8. -/
def WellFormed (ps : List Profile) (c : Nat) : Prop := sorry

-- == Properties ============================================================

/-- Q1. The ChargePointMaxProfile result is the prevailing
ChargePointMaxProfile's limit. -/
theorem q1 (ps : List Profile) (c : Nat) (t : Time) :
    cpMaxResult ps c t = purposeLimit ps c .chargePointMax t := by
  sorry

/-- Q2. A prevailing TxProfile's limit, otherwise the prevailing
TxDefaultProfile's limit. -/
theorem q2 (ps : List Profile) (c : Nat) (t : Time) :
    txResult ps c t =
      match purposeLimit ps c .tx t with
      | some x => some x
      | none => purposeLimit ps c .txDefault t := by
  sorry

/-- Q3. The composite is the minimum of the Q1 and Q2 results that exist. -/
theorem q3 (ps : List Profile) (c : Nat) (t : Time) :
    composite ps c t =
      match cpMaxResult ps c t, txResult ps c t with
      | some m, some x => some (min m x)
      | some m, none => some m
      | none, some x => some x
      | none, none => none := by
  sorry

/-- I1. The composite never exceeds the ChargePointMaxProfile result. -/
theorem i1 (ps : List Profile) (c : Nat) (t : Time) (m : Limit)
    (h : cpMaxResult ps c t = some m) :
    ∃ x, composite ps c t = some x ∧ x ≤ m := by
  sorry

/-- I2. While a TxProfile prevails, TxDefaultProfiles do not affect the
composite. -/
theorem i2 (ps ps' : List Profile) (c : Nat) (t : Time)
    (hTx : (prevailing ps c .tx t).isSome)
    (hSame : ps.filter (·.purpose != .txDefault) = ps'.filter (·.purpose != .txDefault)) :
    composite ps c t = composite ps' c t := by
  sorry

/-- I3. The composite never exceeds the Q2 result. -/
theorem i3 (ps : List Profile) (c : Nat) (t : Time) (x : Limit)
    (h : txResult ps c t = some x) :
    ∃ y, composite ps c t = some y ∧ y ≤ x := by
  sorry

/-- I4. A profile below the prevailing stack level of its purpose does not
affect the composite. -/
theorem i4 (ps : List Profile) (c : Nat) (t : Time) (q p : Profile)
    (hp : prevailing ps c q.purpose t = some p)
    (hlow : q.stackLevel < p.stackLevel) :
    composite (q :: ps) c t = composite ps c t := by
  sorry

end CrosscheckModel.Ocpp16Composite
