/-!
# RQ-1.1: a stored price is one the payment provider accepts

The vision's chain is goal → requirement → spec → code. This file holds the
middle link for one CGV-checked field, `Price.amount` in
`cgv/test_fixtures/tb_max_digits/models.py`:

* `Requirement` is stated in terms of the world: the prices in the database and
  what the payment provider accepts.
* `Spec` is stated at the code boundary: the values the application writes to
  `Price.amount`. Its two clauses are two of the three contracts CGV extracts from
  `DecimalField(max_digits=5, decimal_places=2)`: precision and range. The
  third, non-null, is not modelled (see `README.md`).
* `StoredWasWritten`, `CurrencyHasCents` and `ProviderLimit` are assumptions
  about the world. No proof can establish them. Each is observed or not, and
  `README.md` says how.
* `requirement_holds` proves `Spec ∧ assumptions → Requirement`.
* The `*_needed` theorems show that each of the five premises is needed: drop
  one and a world exists where the rest hold and the requirement fails.

`README.md` beside this package traces the CGV contract to `Spec` and on to
`Requirement`, with the strength of each link.
-/

namespace Requirements.PriceAmount

/-- A finite Python `Decimal`, read as `coeff / 10 ^ places`.
`Decimal("999.99")` is `⟨99999, 2⟩`. -/
structure Dec where
  coeff : Int
  places : Nat
  deriving DecidableEq, Repr

/-- What the requirement and the assumptions talk about. -/
structure World where
  /-- The amounts in the `Price.amount` column of the database. -/
  stored : List Dec
  /-- The amounts the application writes to `Price.amount` on the data paths
  that CGV checks. -/
  written : List Dec
  /-- The ISO 4217 minor-unit exponent of the currency the prices are in. -/
  currencyExponent : Nat
  /-- The payment provider does not refuse a charge or refund of `n` minor
  units on account of its size. -/
  providerAcceptsSize : Int → Prop

/-! ## The requirement, in world terms -/

/-- **R.** Every stored price is a whole number `n` of the currency's minor
units, and the payment provider does not refuse a charge or refund of `n`
minor units for its size. -/
def Requirement (w : World) : Prop :=
  ∀ a ∈ w.stored, ∃ n : Int,
    n * 10 ^ a.places = a.coeff * 10 ^ w.currencyExponent ∧ w.providerAcceptsSize n

/-! ## The spec of the code boundary -/

/-- **S1**, CGV's `precision` contract from `decimal_places=2`: at most two
decimal places. -/
def SpecPrecision (a : Dec) : Prop := a.places ≤ 2

/-- **S2**, CGV's `range` contract from `max_digits=5, decimal_places=2`:
`-999.99 ≤ a ≤ 999.99`, cross-multiplied so that it stays in `Int`. -/
def SpecRange (a : Dec) : Prop :=
  -99999 * 10 ^ a.places ≤ a.coeff * 100 ∧ a.coeff * 100 ≤ 99999 * 10 ^ a.places

/-- **S.** Every amount written to `Price.amount` meets S1 and S2. -/
def Spec (w : World) : Prop := ∀ a ∈ w.written, SpecPrecision a ∧ SpecRange a

/-! ## The assumptions about the world

None of these can be proved. Each is a claim about the world, observed or not. -/

/-- **A1.** Every stored amount reached the column through a write on a data
path CGV checks, and the database keeps it exactly. No raw SQL, migration,
admin edit or second application writes the column. -/
def StoredWasWritten (w : World) : Prop := ∀ a ∈ w.stored, a ∈ w.written

/-- **A2.** The currency's minor unit is a hundredth of its major unit, as for
GBP, EUR and USD. -/
def CurrencyHasCents (w : World) : Prop := w.currencyExponent = 2

/-- **A3.** The payment provider does not refuse for its size any charge or
refund of at most 99,999,999 minor units. -/
def ProviderLimit (w : World) : Prop :=
  ∀ n : Int, -99999999 ≤ n → n ≤ 99999999 → w.providerAcceptsSize n

/-! ## The proof: spec and assumptions give the requirement -/

/-- The minor units of an amount with at most two places: `coeff · 10^(2 - places)`. -/
def minorUnits (a : Dec) : Int := a.coeff * 10 ^ (2 - a.places)

theorem requirement_holds (w : World)
    (hS : Spec w) (hA1 : StoredWasWritten w) (hA2 : CurrencyHasCents w)
    (hA3 : ProviderLimit w) : Requirement w := by
  intro ⟨c, p⟩ ha
  obtain ⟨hp, hlo, hhi⟩ := hS _ (hA1 _ ha)
  simp only [CurrencyHasCents] at hA2
  simp only [SpecPrecision] at hp
  refine ⟨minorUnits ⟨c, p⟩, ?_, hA3 _ ?_ ?_⟩ <;>
  · simp only [minorUnits, hA2] at hlo hhi ⊢
    rcases (by omega : p = 0 ∨ p = 1 ∨ p = 2) with rfl | rfl | rfl <;>
      simp at hlo hhi ⊢ <;> omega

/-! ## Each premise is needed

For each premise, a world where the other four hold and `Requirement` fails.
`0.01` is `⟨1, 2⟩`, `0.001` is `⟨1, 3⟩`, and `1,000,000` is `⟨1000000, 0⟩`. -/

/-- No `n` gives `n · 10^p = c · 10^e` when `c · 10^e` is not a multiple of `10^p`. -/
private theorem not_requirement_single (w : World) (a : Dec) (hs : w.stored = [a])
    (h : ∀ n : Int, n * 10 ^ a.places = a.coeff * 10 ^ w.currencyExponent →
      ¬ w.providerAcceptsSize n) :
    ¬ Requirement w := by
  intro hR
  obtain ⟨n, hn, hacc⟩ := hR a (by simp [hs])
  exact h n hn hacc

/-- Without A1, a value no checked path wrote can sit in the column. -/
theorem storedWasWritten_needed : ∃ w, Spec w ∧ CurrencyHasCents w ∧ ProviderLimit w ∧
    ¬ StoredWasWritten w ∧ ¬ Requirement w := by
  refine ⟨⟨[⟨1, 3⟩], [], 2, fun _ => True⟩, by simp [Spec], rfl, fun _ _ _ => trivial,
    fun h => by simpa using h ⟨1, 3⟩ (by simp), ?_⟩
  exact not_requirement_single _ ⟨1, 3⟩ rfl (fun n hn _ => by simp at hn; omega)

/-- Without A2, a currency with no minor unit (exponent 0, as for JPY) cannot
carry `0.01`. -/
theorem currencyHasCents_needed : ∃ w, Spec w ∧ StoredWasWritten w ∧ ProviderLimit w ∧
    ¬ CurrencyHasCents w ∧ ¬ Requirement w := by
  refine ⟨⟨[⟨1, 2⟩], [⟨1, 2⟩], 0, fun _ => True⟩, ?_, by simp [StoredWasWritten],
    fun _ _ _ => trivial, by simp [CurrencyHasCents], ?_⟩
  · simp [Spec, SpecPrecision, SpecRange]
  · exact not_requirement_single _ ⟨1, 2⟩ rfl (fun n hn _ => by simp at hn; omega)

/-- Without A3, a provider that refuses every amount refuses `0.01`. -/
theorem providerLimit_needed : ∃ w, Spec w ∧ StoredWasWritten w ∧ CurrencyHasCents w ∧
    ¬ ProviderLimit w ∧ ¬ Requirement w := by
  refine ⟨⟨[⟨1, 2⟩], [⟨1, 2⟩], 2, fun _ => False⟩, ?_, by simp [StoredWasWritten], rfl,
    fun h => h 1 (by decide) (by decide), ?_⟩
  · simp [Spec, SpecPrecision, SpecRange]
  · exact not_requirement_single _ ⟨1, 2⟩ rfl (fun _ _ h => h)

/-- Without S1, `0.001` is not a whole number of cents. -/
theorem specPrecision_needed : ∃ w, (∀ a ∈ w.written, SpecRange a) ∧ StoredWasWritten w ∧
    CurrencyHasCents w ∧ ProviderLimit w ∧ ¬ Requirement w := by
  refine ⟨⟨[⟨1, 3⟩], [⟨1, 3⟩], 2, fun _ => True⟩, ?_, by simp [StoredWasWritten], rfl,
    fun _ _ _ => trivial, ?_⟩
  · simp [SpecRange]
  · exact not_requirement_single _ ⟨1, 3⟩ rfl (fun n hn _ => by simp at hn; omega)

/-- Without S2, `1,000,000` is 100,000,000 cents, one more than A3 covers, and
this provider refuses it. -/
theorem specRange_needed : ∃ w, (∀ a ∈ w.written, SpecPrecision a) ∧ StoredWasWritten w ∧
    CurrencyHasCents w ∧ ProviderLimit w ∧ ¬ Requirement w := by
  refine ⟨⟨[⟨1000000, 0⟩], [⟨1000000, 0⟩], 2, fun n => n ≤ 99999999⟩, ?_,
    by simp [StoredWasWritten], rfl, fun _ _ h => h, ?_⟩
  · simp [SpecPrecision]
  · exact not_requirement_single _ ⟨1000000, 0⟩ rfl (fun n hn h => by simp at hn h; omega)

/-! ## The fixture's values against the spec

These read S1 and S2 against the values in the fixture's `ok.py` and `bad.py`.
They agree with CGV's verdicts on that fixture: `ok.py` passes and the two
`Price.amount` writes in `bad.py` fail the range contract. -/

example : SpecPrecision ⟨99999, 2⟩ ∧ SpecRange ⟨99999, 2⟩ := by
  unfold SpecPrecision SpecRange; decide -- 999.99
example : SpecPrecision ⟨-99999, 2⟩ ∧ SpecRange ⟨-99999, 2⟩ := by
  unfold SpecPrecision SpecRange; decide -- -999.99
example : ¬ SpecRange ⟨12345678, 2⟩ := by unfold SpecRange; decide -- 123456.78
example : ¬ SpecRange ⟨-1000, 0⟩ := by unfold SpecRange; decide -- -1000

end Requirements.PriceAmount
