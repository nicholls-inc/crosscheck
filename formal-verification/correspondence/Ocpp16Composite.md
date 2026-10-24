# Correspondence: Ocpp16Composite

Pipeline step: 3 of 5. Seeded by /lean-impl, classified by /correspondence-review (3b.5).

This module differs from the pipeline's usual shape. The Lean model is the oracle for the OCPP 1.6 spec, so its "source" is the spec text and the signed informal spec, not a code file. The system under test in /drt-oracle is EVerest `lib/everest/ocpp/lib/ocpp/v16/profile.cpp` at `dd696165`, driven through `formal-verification/tests/ocpp16_composite/everest_driver/`. Correspondence here asks whether each definition says what the spec says.

## Modelling decisions

- **Time is `Nat` seconds of UTC in calendar days.**
  - Why. D3 and RecurrencyKindType say a daily schedule restarts "at the same time as in the startSchedule". Calendar days make that exact.
  - Risk. An implementation that counts leap seconds disagrees by one second per leap second since startSchedule. That is a finding, not a model error, because the spec wording is about clock time (OCA note N4).
- **Limits are `Nat` tenths of one unit.**
  - Why. 7.14 allows at most one decimal place, and P5 puts every profile in a case in one unit.
  - Risk. Amps and watts are never converted, so conversion bugs are out of reach.
- **`Kind` has no Relative constructor.**
  - Why. P3 puts Relative profiles out of scope, and the type makes them unrepresentable.
  - Risk. Relative profiles are not tested.
- **Ties in stack level keep the first candidate in list order (`pickHighest`).**
  - Why. P6 forbids two applicable profiles of the same purpose, connector and stack level, so ties cannot occur in scope.
  - Risk. The model picks a winner even for inputs outside P6. Generated cases must respect P6, or a divergence may be a generator error.
- **`limitAt` returns none when the schedule does not cover t.**
  - Why. D5 is defined only where D4 holds.
  - Risk. None found. `prevailing` already requires coverage.
- **`txResult` switches on the prevailing TxProfile, not on its limit.**
  - Why. Q2 reads "the limit at t of the prevailing TxProfile if one prevails". The /lean-spec stub stated q2 through `purposeLimit`, which falls back to TxDefault when a prevailing TxProfile has no period at t. /lean-impl corrected the statement to match Q2. I2 is false under the stub's version.
  - Risk. Under P4 a covering schedule always has a period at offset 0, so the two versions agree in scope.

## Definitions to classify

| Lean identifier | Source | Spec behaviour | Pattern |
| --- | --- | --- | --- |
| `applicable` | §3.13.1, D1 | Which connectors each purpose may sit on | Pure transliteration |
| `validAt` | §7.8, D2, A4 | Half-open validFrom to validTo | Pure transliteration |
| `recurrence` | RecurrencyKindType | 86400 for Daily, 604800 for Weekly | Pure transliteration |
| `scheduleStart` | ChargingProfileKindType, D3, A5, A6 | Latest restart at or before t, none before startSchedule | Pure transliteration |
| `covers` | §7.13, D4 | Start exists and t is before start plus duration | Pure transliteration |
| `limitAt` | §7.14, D5 | Last period starting at or before the offset | Pure transliteration |
| `candidates` | §3.13.2, D6 | Applicable, valid and covering profiles of a purpose | Pure transliteration |
| `pickHighest` | §3.13.2, D6, P6 | Highest stackLevel | Pure transliteration |
| `prevailing` | §3.13.2, D6, A1 | `pickHighest` of `candidates` | Pure transliteration |
| `purposeLimit` | D5, D6 | Limit of the prevailing profile | Pure transliteration |
| `cpMaxResult` | §3.13.1, Q1 | ChargePointMaxProfile limit | Pure transliteration |
| `txResult` | §3.13.1, Q2, A2 | TxProfile limit where one prevails, else TxDefault | Pure transliteration |
| `minOpt` | §3.13.3 | Minimum where none means no limit | Pure transliteration |
| `composite` | §3.13.3, Q3 | `minOpt` of the two results | Pure transliteration |
| `conforms` | §3.13.3, Q4, A10 | Implementation at or below the model | Pure transliteration |
| `WellFormed` | P1 to P8 | Preconditions as a `Prop` | Pure transliteration |

## Open questions for /correspondence-review

- `scheduleStart` for Weekly uses 604800-second steps from startSchedule. That matches "at the same time and day-of-the-week as in the startSchedule" only because calendar weeks have no leap seconds in this time model. Confirm that reading.
- `WellFormed` does not state P5 (one unit, at most one decimal), because units are not in the model. Generated cases must enforce it.
- `WellFormed` states P6 only between applicable profiles of the same connectorId. Confirm that is the 3.13.2 replacement rule as signed.

## Status of the theorems

- `q1`, `q2`, `q3`, `i1`, `i2`, `i3` and the lemma `candidates_eq_of_same_nonDefault` are proved. `check_ocpp16_composite.sh` confirms they use only `propext` and `Quot.sound`.
- `i4` is stated and not proved. The plan's step S2 requires I1 and I2 only.
- The five worked examples of the informal spec run as `#guard` checks at build time and pass.

## Source files modelled

- OCPP 1.6 edition 2 FINAL (2017-09-28), sections 3.13.1 to 3.13.3, 7.8 to 7.14 and the ChargingProfileKindType, ChargingProfilePurposeType and RecurrencyKindType enumerations.
- `formal-verification/specs/ocpp16_composite_informal.md`, signed 2026-10-24.
