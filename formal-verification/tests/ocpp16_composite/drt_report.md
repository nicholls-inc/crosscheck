# DRT report: Ocpp16Composite

Run: 2026-10-24
RNG seed: 20261024 (main runs), 1 (mutation self-test)
Inputs: 2,000 cases per configuration. Each case is checked at every instant where the model or EVerest may change value, and one second either side of it.
D4 case: (a). Hand-written production code (EVerest) against a Lean model. Inferred from the correspondence doc, which models the OCPP spec and names one C++ system under test.
Correspondence doc: `formal-verification/correspondence/Ocpp16Composite.md`, revised verdict exact 20, abstraction 4, approximation 0, mismatch 0.
SUT: EVerest `lib/everest/ocpp/lib/ocpp/v16/profile.cpp` at `dd696165`, unmodified, via `everest_driver/`.
Oracle: `CrosscheckModel.Ocpp16Composite` through `Ocpp16CompositeRunner.lean`, Lean v4.28.0.
Conformance rule: Q4 of the signed spec. EVerest must be at or below the model. Above by more than 0.05 of the unit is a fail, below by more than 0.05 is a warn.
Regenerating: a harness run does not overwrite this report. It writes its own to the gitignored `drt_report_latest.md` unless `--report` says otherwise, and refuses to start unless the `leapseconds` file in the tz directory the driver reads lists exactly the three leap seconds since 2010.

## Verdict summary

| Configuration | Pass | Warn | Fail |
| --- | --- | --- | --- |
| Recurring anchors 2017 to 2025 | 2000 | 0 | 0 |
| Recurring anchors 2010 to 2016, across leap seconds | 860 | 67 | 1073 |
| Same 2010 to 2016 cases with `--leap-correct` | 2000 | 0 | 0 |
| Self-test `--mutate-stack` (300 cases, seed 1) | 181 | 34 | 85 |

| Status | Count |
| --- | --- |
| Passed (no divergence) | Every case without a pre-2017 recurring anchor |
| Failed (divergence found) | 1 distinct divergence, in 1,140 cases |
| Skipped (approximation) | 0 |
| Excluded (fully Dafny-verified) | 0 |

## What was compared

Conformance is the Q4 inequality, EVerest at or below the model, so a pass does not mean the two agree at every instant. The harness counts the instants of each kind (`drt_harness.py` prints these counters and writes them to its report). Measured 2026-10-27 at `dd696165`. "Exact" means within 0.05 of the unit.

| Counter | 2017-2025 anchors | 2010-2016 anchors | 2010-2016, `--leap-correct` | Seed 1, 300 cases |
| --- | --- | --- | --- | --- |
| Cases | 2000 | 2000 | 2000 | 300 |
| Instants compared | 67,530 | 78,263 | 67,537 | 10,152 |
| Model gives a limit | 57,304 | 66,541 | 56,896 | 8,439 |
| EVerest exact | 57,304 | 56,902 | 56,896 | 8,439 |
| EVerest below the model | 0 | 4,331 | 0 | 0 |
| EVerest above the model, or no limit where the model has one | 0 | 5,308 | 0 | 0 |
| EVerest gives a limit where the model gives none | 0 | 1,081 | 0 | 0 |
| Neither gives a limit | 10,226 | 10,641 | 10,641 | 1,713 |
| Cases where the model gives no limit at any instant | 55 | 64 | 64 | 18 |

The first three columns use seed 20261024 and 2,000 cases. The 2017 to 2025 run passes 2,000 of 2,000 and EVerest is exact at every one of the 57,304 instants where the model gives a limit. Instants where the model gives no limit are compared too: EVerest also gives none at all 10,226 of them. The 55 cases where the model gives no limit anywhere pass vacuously and are counted above. In the 2010 to 2016 run, the 1,081 instants where EVerest gives a limit and the model gives none are an extra kind of divergence that the harness neither fails nor warns on. It is not zero, see the minimal case below. With `--leap-correct` it returns to zero.

The seed 1 column is a 300-case run of the same generator with no flags, which reproduces the earlier one-off figures of 10,152 and 8,439. Cases average 3.4 profiles (up to 7), across all three purposes and the Absolute, Daily and Weekly kinds. The `--mutate-stack` self-test on the same 300 cases gives 6,742 exact, 795 below and 902 above, so the counters do separate agreement from disagreement.

### Minimal case, EVerest limit where the model gives none (2010-2016 run)

Case 2 of seed 20261024 with `--old-anchors`, minimised. One TxProfile on connector 1, stackLevel 0, Recurring Weekly, startSchedule 2013-01-23T00:00:00Z (1358899200), duration 172800 s, one period (0 s, 12.1 A). Query window from 2026-03-10T22:15:00Z for 48 hours. At 2026-03-10T23:59:58Z and 23:59:59Z the model gives no limit and EVerest gives 12.1 A. Measured: with `--leap-correct` no such instant remains in the 2,000 cases. Inferred, not checked on this case alone: this is the same leap-second shift as the failures below, here at the weekly restart. The harness does not classify this kind, and the operator has not yet said whether it should fail.

## Failures

### composite — daily and weekly boundaries drift one second per leap second

- **D4 case applied.** (a).
- **Witness (minimised).** One TxDefaultProfile on connector 0, stackLevel 3, Recurring Daily, startSchedule 2016-09-14T11:00:00Z (1473850800), no duration, periods (0 s, 9249.9 W) and (40500 s, 6543.6 W). Window 2026-08-13T03:00:00Z for 24 hours. At 2026-08-13T10:59:59Z the model gives 6543.6 W, because the day's second period still applies. EVerest gives 9249.9 W, because it restarts the daily schedule one second before 11:00:00, the time the spec says the schedule restarts ("at the same time as in the startSchedule"). EVerest is above the model, so this is a fail. Where the restart lowers the limit instead, the same drift shows as a warn.
- **Witness (original).** Seed 20261024 with `--old-anchors`, case 3. Replay with `python3 formal-verification/tests/ocpp16_composite/drt_harness.py --seed 20261024 --count 2000 --old-anchors --replay 3`.
- **Attribution.** With `--leap-correct`, every Recurring anchor in EVerest's input moves forward by the leap seconds since it (2012-06-30, 2015-06-30 and 2016-12-31). All 2,000 cases then pass, so leap seconds explain all 1,140 flagged cases. The S1 controls give the same answer. The section 3.13.7 example switches at 28798 s from a 2013 anchor, 28799 s from 2016-07-01 and 28800 s from 2020-01-01.
- **Mechanism.** `profile.cpp:207-210` takes `(now − startSchedule) % 86400` on `date::utc_clock` instants (`common/types.cpp:14-53`), and that clock counts leap seconds.
- **Divergence classification.** Production-code gap. The source computes recurrence in elapsed SI seconds where the spec asks for the same clock time. The spec wording is open to the other reading, so it is also OCA note N4.
- **Severity.** Low. The shift is one to three seconds today and grows by one second per future leap second. Physical impact is negligible, but a charger reports and enforces limits early, at times its central system did not ask for.
- **Reproducer.** `python3 formal-verification/tests/ocpp16_composite/drt_harness.py --count 2000 --seed 20261024 --old-anchors` reports the failures. `everest_driver/examples/leap_control_2016.json` and `leap_control_2020.json` show it on one profile.

Minimised witnesses for six more of the flagged cases are in `drt_witnesses_old_anchors.md`.

## Passes

- `composite` and every definition it reaches, for Recurring anchors from 2017 to 2025: 2,000 cases, seed 20261024, no divergence.
- No correspondence-error divergences. The revised exact and abstraction regions held on every case.

## Skips (approximation)

None. The reviewer classified `candidates` and `txResult` as approximation. The pipeline owner reclassified them as abstraction because they follow the operator's signed resolutions A1 and A2. Neither region produced a divergence, so the reclassification did not change any result.

## Exclusions (fully Dafny-verified slices)

None.

## Outside the generator, by design

- **TxDefaultProfiles on connector 0 and connector 1 together (A3).** The model takes no position, so the generator never mixes them. A direct probe of EVerest gives 10 A for connector 0 at 10 A stack 5 with connector 1 at 20 A stack 1, 20 A with the stack levels swapped, and 20 A when both are stack 3. EVerest lets stackLevel decide, and the connector-specific profile replaces the connector-0 one only at an equal stack level. The equal-stack result runs through the driver's copy of EVerest's installation glue, cited in `everest_driver/README.md`. This is evidence for OCA note N2.
- **Relative profiles, phases, amp and watt conversion, offline behaviour and transactionId matching.** Out of scope per the signed spec.

## Triage (S4)

| Divergence | Class | Next step |
| --- | --- | --- |
| Recurrence drift of one second per leap second | Production-code gap, plus spec wording (N4) | Draft an EVerest issue with the witness. File only after human review, per EVerest's contribution policy for AI-assisted work. Include in the OCA report as N4. |

No model errors were found in DRT. Model and spec errors found earlier by /correspondence-review (CR-OC16-01, CR-OC16-02) were fixed before these runs.

Absence of other divergences is evidence, not proof. The generator bounds are at most 2 ChargePointMaxProfiles, 3 TxDefaultProfiles and 2 TxProfiles per case, at most 4 periods each, 15-minute period offsets, 24- or 48-hour windows in 2026, and connector 1.

## Draft EVerest issue (not filed)

For a human to review, edit and file at https://github.com/EVerest/EVerest/issues. EVerest's contributor policy requires every AI-assisted contribution to be reviewed and understood by a human first.

> **Title.** fix(ocpp): OCPP 1.6 Recurring charging profiles restart early by one second per leap second
>
> **What happens.** For a Recurring Daily or Weekly profile, `calculate_start` in `lib/everest/ocpp/lib/ocpp/v16/profile.cpp` computes the restart as `(now − startSchedule) % 86400` (or 604800) on `date::utc_clock` time points. `utc_clock` counts leap seconds, so each leap second between `startSchedule` and now moves every restart one second earlier.
>
> **Expected.** OCPP 1.6 edition 2, RecurrencyKindType: Daily "restarts every 24 hours, at the same time as in the startSchedule". With startSchedule 2013-01-01T00:00:00Z and the section 3.13.7 example profile, the 6 kW period should start at 08:00:00.
>
> **Observed** at `dd696165`, with `profile.cpp` driven directly:
>
> | startSchedule | Leap seconds since | Switch to 6 kW |
> | --- | --- | --- |
> | 2013-01-01 | 2 | 07:59:58 |
> | 2016-07-01 | 1 | 07:59:59 |
> | 2020-01-01 | 0 | 08:00:00 |
>
> **Why the tests miss it.** The v16 profile tests use 2023 and 2024 dates, after the last leap second (2016-12-31).
>
> **Impact.** Limits switch one to three seconds early today, by one more second per future leap second, for any central system that anchors recurring profiles before 2017.
>
> **Possible fix.** Do the recurrence arithmetic on `date::sys_time` (or convert with `clock_cast<system_clock>`) so that a day is 86400 clock seconds.
