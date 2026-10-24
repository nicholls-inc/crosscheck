# Informal specification: ocpp16_composite

Source: OCPP 1.6 edition 2 FINAL, Open Charge Alliance, 2017-09-28, sections 3.13.1 to 3.13.3, 3.13.7 and 7.8 to 7.14 (including the ChargingProfileKindType, ChargingProfilePurposeType and RecurrencyKindType enumerations). Quotes are verbatim. Everything else is a description in original words with the section it comes from.

## Fitness assessment

| Prerequisite | Verdict | Why |
| --- | --- | --- |
| #1 Deterministic algebraic semantics | Pass | The composite limit at a time t is a function of the installed profiles, the connector and t. The wall-clock time is an input, not hidden state. |
| #2 Provable properties | Pass | "The composite never exceeds the prevailing ChargePointMaxProfile limit" and "a prevailing TxProfile hides every TxDefaultProfile" are quantified statements over all profile sets and times. |
| #3 Tractable input generation | Pass | Profile sets are small finite records (purpose, stack level, kind, validity, a few periods). Random generation within stated bounds covers the space. |
| #4 Dual-development resources | Flagged (D6 hypothesis) | Untested working assumption, not a gate. |

The module is a fit. No override.

## Module boundary

In scope:
- `composite(profiles, c, t)`, the limit that applies to connector c at time t, for one connector c ≥ 1 during an active transaction on c, with every profile in one charging rate unit.
- The helper notions it is defined from. These are profile validity, schedule start, schedule coverage, the limit of a profile at t, the prevailing profile of a purpose, and the Tx or TxDefault result.

Out of scope, and so not specified here:
- Relative profiles, `numberPhases`, `minChargingRate`, conversion between A and W, and the station-wide composite for connector 0, where limits are summed over connectors.
- Installing, replacing and deleting profiles (SetChargingProfile, ClearChargingProfile and the replacement rule of 3.13.2). The input is the set of profiles already installed.
- What a charger does when no profile limits it.
- Offline behaviour (3.13.6) and anything that happens outside a transaction.

Assumed callee contracts: none. The module calls nothing.

## Input shape

(c) Signature plus prose intent, where the prose intent is the OCPP 1.6 edition 2 text listed under Source. The text is authoritative. No implementation was used as a source.

## Definitions

These terms are used by the conditions below.

- D1. **Applicable profile.** A ChargePointMaxProfile is applicable if its connectorId is 0. A TxDefaultProfile is applicable if its connectorId is 0 or c. A TxProfile is applicable if its connectorId is c. (3.13.1)
- D2. **Valid at t.** A profile is valid at t when validFrom ≤ t, if validFrom is present, and t < validTo, if validTo is present. If validFrom is absent it is valid from receipt, and if validTo is absent it is valid until replaced (7.8). The half-open boundary is resolution A4.
- D3. **Schedule start at t.** For an Absolute profile, it is `startSchedule` ("Schedule periods are relative to a fixed point in time defined in the schedule", 7.9). For a Recurring profile with recurrencyKind Daily, it is the latest time `startSchedule + k × 86400` with k ≥ 0 that is not after t, and for Weekly the same with 604800 ("The schedule restarts every 24 hours, at the same time as in the startSchedule", and every 7 days for Weekly). Time here is UTC counted in calendar days, so a day is always 86400 seconds and leap seconds are not counted. A Recurring profile has no schedule start before its startSchedule (resolution A5).
- D4. **Covers t.** A profile's schedule covers t when it has a schedule start s at t, and either `duration` is absent or t < s + duration (7.13). "If the duration is left empty, the last period will continue indefinitely." For a Recurring profile, D3 picks the latest occurrence, so without a duration its last period runs until the next restart (resolution A6).
- D5. **Limit of a profile at t.** It is the `limit` of the last period whose startPeriod ≤ t − s. "The value of StartPeriod also defines the stop time of the previous period." (7.14)
- D6. **Prevailing profile of a purpose at t.** Among the applicable profiles of that purpose that are valid at t and whose schedule covers t, it is the one with the highest stackLevel (3.13.2). A profile whose schedule does not cover t never prevails (resolution A1).

## Preconditions

- P1. c ≥ 1, and a transaction is active on c throughout the period being evaluated.
- P2. Every profile has stackLevel ≥ 0, a purpose of ChargePointMaxProfile, TxDefaultProfile or TxProfile, and a connectorId as allowed by 3.13.1. A ChargePointMaxProfile has connectorId 0 and a TxProfile has connectorId > 0. Profiles that are not applicable to c (D1) may be present and have no effect.
- P3. Every profile is Absolute or Recurring. Every Recurring profile has a recurrencyKind. Every profile has a `startSchedule`.
  - Satisfies: an Absolute profile with startSchedule 2026-01-01T00:00:00Z.
  - Violates: a Relative profile, or an Absolute profile with no startSchedule.
- P4. Every schedule has at least one period. The first period's startPeriod is 0 (7.13, see ambiguity A8). startPeriod values strictly increase.
- P5. Every profile uses the same chargingRateUnit. Every limit is ≥ 0 with at most one decimal place (7.14).
  - Satisfies: limits 16, 6.5 and 0 in A.
  - Violates: 6.25 A, or one profile in A and another in W.
- P6. No two applicable profiles have the same purpose, stackLevel and connectorId (3.13.2 replacement rule). TxDefaultProfiles are installed on connector 0 or on connector c, never on both (resolution A3). The spec does not settle how the two interact, so the model takes no position and generated cases avoid the mix.
- P8. No TxProfile carries a transactionId (resolution A9).
- P7. If `duration` is present it is > 0.

## Postconditions

- Q1. The ChargePointMaxProfile result at t is the limit at t (D5) of the prevailing ChargePointMaxProfile (D6), or nothing if none prevails.
- Q2. The Tx result at t is the limit at t of the prevailing TxProfile if one prevails at t. Otherwise it is the limit at t of the prevailing TxDefaultProfile, or nothing if neither prevails. TxProfile "SHALL overrule the default charging profile with purpose TxDefaultProfile for the duration of the current transaction only" (3.13.1). The override applies only at instants where a TxProfile prevails (resolution A2).
- Q3. `composite(profiles, c, t)` is the smaller of the Q1 and Q2 results where both exist, whichever one exists if only one does, and nothing if neither does. "This Composite Schedule is calculated by taking the minimum value for each time interval" (3.13.3).
- Q4. Conformance of an implementation. An implementation's limit at t conforms when it is at or below `composite(profiles, c, t)`, or anything when the composite is nothing. The composite "SHALL be less than or equal to lowest value of available power or current in any of the merged schedules" (3.13.3). The model computes the minimum itself, the largest value allowed (resolutions A7 and A10).

## Invariants

- I1. For every profile set, c and t, if the Q1 result is m then composite(profiles, c, t) exists and is ≤ m.
- I2. For every profile set, c and t, if a TxProfile prevails at t, then adding, removing or changing TxDefaultProfiles leaves composite(profiles, c, t) unchanged.
- I3. For every profile set, c and t, if the Q2 result is x then composite(profiles, c, t) exists and is ≤ x.
- I4. For every profile set, c and t, adding a profile whose purpose already has a prevailing profile at t with a higher stackLevel, and which does not itself prevail at t, leaves composite(profiles, c, t) unchanged.

## Termination

Every function is a finite traversal of the finite profile list and the finite period list of one profile. The recurrence index k is computed by integer division, not by iteration. No recursion beyond list traversal.

## Edge cases

- No profiles. The composite is nothing.
- No applicable profiles (all on other connectors). The composite is nothing.
- Only a ChargePointMaxProfile. The composite is its limit.
- A limit of 0. The composite is 0 wherever that profile prevails and is the smaller.
- t exactly at a startPeriod boundary. The new period's limit applies (7.14: startPeriod "defines the stop time of the previous period").
- t exactly at schedule start + duration. The schedule no longer covers t (D4).
- t exactly at validFrom. The profile is valid. At exactly validTo it is not (A4).
- t before startSchedule of a Recurring profile. The profile does not cover t (A5).
- A Recurring profile whose duration is absent or longer than its recurrence period. The latest occurrence that has started applies (A6).
- A TxProfile that is installed but whose schedule does not cover t. It does not prevail, and the prevailing TxDefaultProfile applies (A1, A2).
- TxDefaultProfiles on both connector 0 and connector c. Outside the precondition P6 (A3).
- A daily or weekly recurrence across a leap second. Boundaries stay at the same UTC clock time (D3).

## Worked examples

All in one unit and for connector 1 during a transaction. Times are UTC on 2026-01-01.

1. The section 3.13.7 example. One TxDefaultProfile on connector 1, stackLevel 0, Recurring Daily, startSchedule 2013-01-01T00:00Z, duration 86400, unit W, periods (0, 11000), (28800, 6000), (72000, 11000). Expected composite: 11000 W from 00:00, 6000 W from 08:00, 11000 W from 20:00. Exercises D3 Daily, D5, Q2 and Q3.
2. A ChargePointMaxProfile at 16 A (connector 0, Absolute from 00:00, no duration) and a TxProfile at 32 A (connector 1, Absolute from 00:00). Expected: 16 A at every t. Exercises Q1, Q3 and I1.
3. TxDefaultProfile A at 10 A (stackLevel 0, Absolute from 00:00, no duration) and TxDefaultProfile B at 20 A (stackLevel 1, Absolute from 10:00, duration 3600). Expected: 10 A before 10:00, 20 A from 10:00 to 11:00, and 10 A from 11:00. Exercises D6, I4 and A1.
4. TxProfile at 6 A (stackLevel 0, Absolute from 00:00, duration 43200) and TxDefaultProfile at 32 A (connector 0, Absolute from 00:00). Expected: 6 A before 12:00 and 32 A from 12:00. Exercises Q2, I2 and A2.
5. ChargePointMaxProfile at 8 A with validTo 12:00, and TxDefaultProfile at 16 A. Expected: 8 A before 12:00 and 16 A after. Exercises D2, Q3, I3 and A4.

## Ambiguities

Each was resolved by the operator on 2026-10-24. A1 to A3 were the operator's own calls, and A4 to A10 accept the recommendations offered at sign-off.

- A1. Can a profile that is valid at t but whose schedule does not cover t prevail? **No.** A profile prevails only where its schedule covers t (D6). OCA note N1.
- A2. Does a TxProfile overrule TxDefaultProfiles only where its schedule covers t, or for the whole transaction? **Only where it covers t** (Q2).
- A3. How do TxDefaultProfiles on connector 0 and connector c interact? **The model takes no position.** Precondition P6 excludes the mix, and generated cases avoid it. OCA note N2.
- A4. Is a profile valid at exactly validFrom and at exactly validTo? **Valid at validFrom, not at validTo** (D2).
- A5. Does a Recurring profile apply before its startSchedule? **No** (D3).
- A6. Which occurrence applies when a Recurring duration is absent or longer than the recurrence? **The latest one that has started** (D3, D4).
- A7. What is the composite where no profile prevails? **Nothing, meaning no limit from the spec.** Any implementation value conforms there (Q4).
- A8. Does 7.13's "startSchedule of the first ChargingSchedulePeriod" mean the first period's startPeriod? **Yes** (P4). OCA note N3.
- A9. Should a TxProfile with a non-matching transactionId be ignored? **Out of scope.** Precondition P8 excludes transactionId.
- A10. Does the model compute the minimum, with conformance at or below it? **Yes** (Q4).

## Notes for the Open Charge Alliance

These are wording problems in OCPP 1.6 edition 2 found while writing this spec. They are recorded for a report, not resolved by it.

- N1. **Prevailing profile (3.13.2).** The normative sentence picks the highest stackLevel among profiles "valid at that point in time, as determined by their validFrom and validTo parameters". Read literally, a profile whose schedule duration has ended still prevails and hides lower levels. The following sentence, "If you use Stacking without a duration, on the highest stack level, the Charge Point will never fall back to a lower stack level profile", implies a charger does fall back once a duration ends. Suggested fix: define prevailing over profiles that are valid and whose schedule covers that point in time.
- N2. **TxDefaultProfile on connector 0 and connector > 0 (3.13.1).** The text says a connector-0 TxDefaultProfile "SHALL be replaced only for that specific connector" when a profile for a specific connector arrives. It does not say whether this applies across stack levels or only at the same stackLevel, where 3.13.2's replacement rule already applies. Implementations differ. EVerest (lib/everest/ocpp at dd696165) pools both sets and lets stackLevel decide.
- N3. **First period (7.13).** "The startSchedule of the first ChargingSchedulePeriod SHALL always be 0" names a field that does not exist on ChargingSchedulePeriod. The intended field is startPeriod.
- N4. **Recurrence and leap seconds (RecurrencyKindType).** "The schedule restarts every 24 hours, at the same time as in the startSchedule" has two readings once a leap second falls between startSchedule and now. Every 86400 elapsed seconds drifts from "the same time". Stating that recurrence follows UTC clock time would remove the doubt. EVerest follows elapsed seconds and drifts one second per leap second.

## Pipeline forward references

- /lean-spec (sub-phase 3b.3) translates this prose into a Lean 4 spec stub with `sorry` proof bodies.
- /lean-impl (sub-phase 3b.4) builds the Lean functional model.
- /correspondence-review (sub-phase 3b.5) classifies model-versus-source correspondence.
- /drt-oracle (sub-phase 3b.6) runs differential random testing against the model. The system under test is EVerest `lib/everest/ocpp/lib/ocpp/v16/profile.cpp` at commit dd696165.
- /intent-check (Layer 5) composes with this prose as the spec arm of the (prose, test, code-diff) triple.

Human sign-off: 2026-10-24
