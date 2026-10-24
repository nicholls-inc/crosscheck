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
- D2. **Valid at t.** A profile is valid at t when t is not before its validFrom, if present, and not after its validTo, if present. If validFrom is absent it is valid from receipt, and if validTo is absent it is valid until replaced (7.8). Boundary inclusion is ambiguity A4.
- D3. **Schedule start at t.** For an Absolute profile, it is `startSchedule` ("Schedule periods are relative to a fixed point in time defined in the schedule", 7.9). For a Recurring profile with recurrencyKind Daily, it is the latest time `startSchedule + k × 86400` that is not after t, and for Weekly the same with 604800 ("The schedule restarts every 24 hours, at the same time as in the startSchedule", and every 7 days for Weekly). Whether k may be negative is ambiguity A5.
- D4. **Covers t.** A profile's schedule covers t when t is not before its schedule start s, and either `duration` is absent or t < s + duration (7.13). "If the duration is left empty, the last period will continue indefinitely."
- D5. **Limit of a profile at t.** It is the `limit` of the last period whose startPeriod ≤ t − s. "The value of StartPeriod also defines the stop time of the previous period." (7.14)
- D6. **Prevailing profile of a purpose at t.** Among the applicable profiles of that purpose that are valid at t, it is the one with the highest stackLevel (3.13.2). Whether a profile that is valid but does not cover t can prevail is ambiguity A1.

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
- P6. No two applicable profiles have the same purpose, stackLevel and connectorId (3.13.2 replacement rule). Whether a TxDefaultProfile on connector 0 and one on connector c may share a stack level is ambiguity A3.
- P7. If `duration` is present it is > 0.

## Postconditions

- Q1. The ChargePointMaxProfile result at t is the limit at t (D5) of the prevailing ChargePointMaxProfile (D6), or nothing if none prevails.
- Q2. The Tx result at t is the limit at t of the prevailing TxProfile if one prevails. Otherwise it is the limit at t of the prevailing TxDefaultProfile, or nothing if neither prevails. TxProfile "SHALL overrule the default charging profile with purpose TxDefaultProfile for the duration of the current transaction only" (3.13.1). Ambiguity A2 asks whether this is per instant.
- Q3. `composite(profiles, c, t)` is the smaller of the Q1 and Q2 results where both exist, whichever one exists if only one does, and nothing if neither does. "This Composite Schedule is calculated by taking the minimum value for each time interval" (3.13.3).
- Q4. Conformance of an implementation. An implementation's limit at t conforms when it is at or below `composite(profiles, c, t)`, or anything when the composite is nothing. The composite "SHALL be less than or equal to lowest value of available power or current in any of the merged schedules" (3.13.3). Ambiguity A10 confirms this reading.

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
- t exactly at validFrom or validTo. Escalated to A4.
- t before startSchedule of a Recurring profile. Escalated to A5.
- A Recurring profile whose duration is absent or longer than its recurrence period, so occurrences overlap. Escalated to A6.
- A TxProfile that is installed but whose schedule does not cover t. Escalated to A1 and A2.
- TxDefaultProfiles on both connector 0 and connector c. Escalated to A3.

## Worked examples

All in one unit and for connector 1 during a transaction. Times are UTC on 2026-01-01.

1. The section 3.13.7 example. One TxDefaultProfile on connector 1, stackLevel 0, Recurring Daily, startSchedule 2013-01-01T00:00Z, duration 86400, unit W, periods (0, 11000), (28800, 6000), (72000, 11000). Expected composite: 11000 W from 00:00, 6000 W from 08:00, 11000 W from 20:00. Exercises D3 Daily, D5, Q2 and Q3.
2. A ChargePointMaxProfile at 16 A (connector 0, Absolute from 00:00, no duration) and a TxProfile at 32 A (connector 1, Absolute from 00:00). Expected: 16 A at every t. Exercises Q1, Q3 and I1.
3. TxDefaultProfile A at 10 A (stackLevel 0, Absolute from 00:00, no duration) and TxDefaultProfile B at 20 A (stackLevel 1, Absolute from 10:00, duration 3600). Expected: 10 A before 10:00, 20 A from 10:00 to 11:00, and 10 A from 11:00, if a profile that no longer covers t cannot prevail (A1). Exercises D6, I4 and A1.
4. TxProfile at 6 A (stackLevel 0, Absolute from 00:00, duration 43200) and TxDefaultProfile at 32 A (connector 0, Absolute from 00:00). Expected: 6 A before 12:00. After 12:00 it is 32 A under a per-instant reading of A2, or nothing under the other reading. Exercises Q2, I2 and A2.
5. ChargePointMaxProfile at 8 A with validTo 12:00, and TxDefaultProfile at 16 A. Expected: 8 A before 12:00 and 16 A after. Exercises D2, Q3, I3 and A4.

## Ambiguities

- A1. Can a profile that is valid at t (D2) but whose schedule does not cover t (D4) be the prevailing profile of its purpose, so hiding lower stack levels? 3.13.2 defines prevailing by validFrom and validTo only, yet says that without a duration "the Charge Point will never fall back to a lower stack level profile".
- A2. Does a TxProfile overrule TxDefaultProfiles only at instants where its own schedule covers t, or for the whole transaction once it is installed?
- A3. When TxDefaultProfiles are installed on both connector 0 and connector c, does a connector-c profile hide every connector-0 profile, only those with the same stackLevel, or do they compete by stackLevel alone? 3.13.1 says the connector-0 profile "SHALL be replaced only for that specific connector".
- A4. Is a profile valid at exactly t = validFrom, and at exactly t = validTo?
- A5. For a Recurring profile and t before its startSchedule, does the recurrence apply (negative k), or does the profile not cover t?
- A6. For a Recurring profile whose duration is absent or longer than its recurrence period, which occurrence's periods apply at t?
- A7. Where no profile prevails for any purpose, is the composite "no limit", or some charger default? It is out of scope here, but an implementation will report a number.
- A8. 7.13 says "The startSchedule of the first ChargingSchedulePeriod SHALL always be 0". Does this mean the first period's startPeriod is 0?
- A9. A TxProfile may carry a transactionId (7.8). Should a TxProfile whose transactionId differs from the active transaction be ignored by the composite?
- A10. Should the model compute the minimum itself (the largest value 3.13.3 allows), with an implementation conforming when it is at or below it?

## Pipeline forward references

- /lean-spec (sub-phase 3b.3) translates this prose into a Lean 4 spec stub with `sorry` proof bodies.
- /lean-impl (sub-phase 3b.4) builds the Lean functional model.
- /correspondence-review (sub-phase 3b.5) classifies model-versus-source correspondence.
- /drt-oracle (sub-phase 3b.6) runs differential random testing against the model. The system under test is EVerest `lib/everest/ocpp/lib/ocpp/v16/profile.cpp` at commit dd696165.
- /intent-check (Layer 5) composes with this prose as the spec arm of the (prose, test, code-diff) triple.
