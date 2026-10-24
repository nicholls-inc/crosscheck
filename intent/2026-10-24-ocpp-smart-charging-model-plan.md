# Plan: OCPP 1.6 composite schedule model, tested against EVerest

Intent: `intent/2026-10-24-ocpp-smart-charging-model.md`. Each step writes its own `spec.md` at its start if `docs/assurance/DEVELOPMENT-FRAMEWORK.md` requires one. No step touches a protected surface.

This plan is the smallest piece of work that tests the program's value claim. The claim is that a proved model of OCPP smart charging finds real disagreements in a widely used open-source charger stack. The plan builds a Lean model of the OCPP 1.6 composite schedule, proves two properties of it, and runs EVerest's 1.6 composite calculation against it on generated inputs. Everything else in the program waits until this produces a result. The deferred work is listed in Appendix B.

## How to read this

One box is one unit of work. Every box names the evidence that checks it. Check a box only when its evidence exists.

Only deterministic checks and human review count as evidence (`docs/VISION.md`). The oracle is the Lean model run through `lean_run`, not a hand-written transcription of it.

Steps run in order. S1 is a spike with a kill criterion, because it carries the most risk.

| Step | Output | Guess |
| --- | --- | --- |
| S1 | EVerest's v16 composite calculation runs from the command line | 1 to 2 days |
| S2 | Lean model of the 1.6 composite schedule with two proved properties | 1 week |
| S3 | Differential test of EVerest against the model, and a report | 3 days |
| S4 | Triage of every divergence | 2 days |

## Run EVerest's composite calculation on its own (S1)

**Depends on.** None.

**Files.**

- [x] Create `formal-verification/tests/ocpp16_composite/everest_driver/`, with a `main.cpp` that reads a profile set and a time window as JSON and prints the composite schedule as JSON. Done in `5801529`.

**Build.**

- [x] Pin `EVerest/EVerest` to a commit and record it in the driver's `README.md`. Pinned at `dd696165` in the driver README.
- [x] Link `lib/everest/ocpp/lib/ocpp/v16/profile.cpp` and only what it needs. Call the function that `v16/smart_charging.cpp` uses to build the composite. The real `profile.cpp` is compiled unmodified. The handler glue from `smart_charging.cpp` is copied into `main.cpp` with line citations, as the README lists.

**You see.**

- [x] The driver prints a composite schedule for the example in section 3.13.7, a daily recurring TxDefaultProfile in watts. `examples/spec_3_13_7.json`.

**Verify.**

- [ ] The output is 11,000 W from 00:00, 6,000 W from 08:00 and 11,000 W from 20:00, as section 3.13.7 describes. Record the command and output in the README. **Not met as written.** EVerest switches at 28798 s and 71998 s, two seconds early. This is the leap-second finding, confirmed by the 2016 and 2020 controls in the README.

**Kill criterion.** If the driver does not build in two days, stop and choose between two fallbacks. One is to add generated cases to EVerest's existing gtest target in `lib/everest/ocpp/tests/lib/ocpp/v16/` (`test_composite_schedule.cpp`). The other is to drive the SIL over OCPP, which costs about a week more.

## Write and prove the composite model in Lean (S2)

**Depends on.** None. It can run alongside S1.

**Files.**

- [x] Create `formal-verification/specs/ocpp16_composite_informal.md`, one page. Signed 2026-10-24.
- [x] Create `formal-verification/lean/CrosscheckModel/Ocpp16Composite.lean`.

**Build.**

- [x] List the rules the model covers, each with its spec section, in the informal spec. Cover profile purposes and connector 0 (3.13.1), stack levels (3.13.2), combining (3.13.3), validity windows, and the Absolute and Recurring kinds. D1 to D6 and P1 to P8.
- [x] Leave Relative profiles, phases and conversion between amps and watts out of this step and say so in the spec. Every profile in one case uses the same unit. Module boundary and P5.
- [x] Name the data shape first. `Limit` is a natural number of tenths of the case's unit, because limits allow one decimal place (7.14). Time is whole seconds. `composite : ProfileSet → Connector → Time → Option Limit`.
- [x] Prove T1. Where a valid ChargePointMaxProfile covers time t, the composite at t is at or below its limit. `i1`, proved in `0ca9a2b`.
- [x] Prove T2. Where a valid TxProfile covers t, no TxDefaultProfile affects the composite at t. `i2`, proved in `0ca9a2b`.
- [x] Make `composite` runnable with `lean_run`, and print the composite for the S1 example. Through `Ocpp16CompositeRunner.lean` with local `lean --run`, because the Docker image behind `lean_run` was unavailable.

**You see.**

- [ ] `lean_check` passes with T1 and T2 listed, and `lean_run` prints the same composite as S1 for the section 3.13.7 example. **Not met as written.** The model builds with `i1` and `i2` proved and prints 08:00 and 20:00 for the 3.13.7 example. S1 prints 07:59:58 and 19:59:58, so the two differ by the leap-second finding. They agree on the 2020 control.

**Verify.**

- [x] `#print axioms` for T1 and T2 shows only `propext`, `Classical.choice` and `Quot.sound`, which rules out `sorry`. `check_ocpp16_composite.sh` shows `i1` uses `propext` and `i2` uses `propext` and `Quot.sound`. It fails on the unproved `i4`.
- [ ] The operator reads the informal spec and the two theorem statements against the PDF and signs the spec's review table. **Open.** The informal spec is signed. The operator has not yet read the theorem statements, or confirmed the D4 correction made after /correspondence-review.

## Test EVerest against the model (S3)

**Depends on.** S1 and S2.

**Files.**

- [ ] Create `formal-verification/tests/ocpp16_composite/drt_harness.py`, `README.md` and `drt_report.md`. **Partly done.** `drt_harness.py` and `drt_report.md` exist, in `404c4c5` and `2697990`. No `README.md` yet. The driver has its own.

**Build.**

- [x] Follow `/drt-oracle`, with the Lean model through `lean_run` as the oracle and the S1 driver as the system under test. With `/correspondence-review` run first, which found two mismatches that were fixed before DRT.
- [x] Generate profile sets with up to three profiles per purpose and up to four periods each, Absolute and Recurring, on connectors 0 and 1. Each case uses one unit, amps or watts. Up to 2 ChargePointMax, 3 TxDefault and 2 Tx profiles per case.
- [x] Compare at every period boundary in the window. Also one second either side of each boundary.
  - [x] Fail when EVerest's limit is above the model's. The spec forbids it.
  - [x] Warn when EVerest's limit is below the model's by more than 0.05 of the unit. The spec allows it, but it means lost charging.
- [x] Shrink each fail and warn to a minimal case. Record the seed, the case count and the EVerest commit. Seeds 20261024 and 1, 2,000 cases, EVerest `dd696165`.

**You see.**

- [x] `drt_report.md` gives pass, warn and fail counts and a minimal case for each distinct divergence.

**Verify.**

- [x] 2,000 cases run in under 15 minutes. The Lean runner per case is the slow part, so batch cases into one `lean_run` call if it is slower. 16 s for 2,000 cases with recent anchors and 45 s with old anchors, with every case batched into one Lean call.
- [x] A mutated driver, with the stack-level comparison reversed, produces fails. This shows the harness can catch a bug. `--mutate-stack` reverses stack levels in EVerest's input instead of rebuilding the driver. It flags 119 of 300 cases.

## Triage every divergence (S4)

**Depends on.** S3.

- [x] Classify each distinct divergence as an EVerest bug, a spec ambiguity or a model error. Fix model errors in S2 and rerun S3. One divergence, an EVerest production-code gap. Model errors CR-OC16-01 and CR-OC16-02 were fixed before the final runs.
- [x] For each EVerest bug, write a reproducing case and a draft issue in `drt_report.md`. Done in `8f8bf40`.
- [x] Write the result in `drt_report.md`, either the bugs found or the case count and bounds within which EVerest agreed with the model.

## Appendix A. Facts this plan relies on

Each was checked against a primary source on 2026-10-24.

- Sections 3.13.1 to 3.13.7 and 7.14 of OCPP 1.6 edition 2 (2017-09-28) say what the plan cites. A limit "accepts at most one digit fraction".
- The composite is an upper bound. Section 3.13.3 says the merged schedule "SHALL be less than or equal to" the lowest limit. This is why S3 fails only on "above".
- EVerest's 1.6 composite is in `lib/everest/ocpp/lib/ocpp/v16/profile.cpp` (719 lines) and is separate from the 2.x copy. It was rewritten in May 2025 (`EVerest/libocpp#1062`), and roughly 15 to 20 smart-charging fixes have been merged across EVerest since 2024.

S1 resolved the main risk. The v16 code links on its own with two shim headers, so the kill criterion was not used.

## Appendix B. Deferred until S4 produces a result

Each item is deferred, not dropped. It returns if S4 shows the approach finds real problems.

- **Quint model of the profile lifecycle.** It covers transactions, reboots and the delete-or-keep SHOULD. It needs a black-box harness to pay off, so it waits with that harness.
- **Black-box harness over OCPP.** This is the industry-facing deliverable, but it costs the most, and S3 tests the same calculation more cheaply.
- **MicroOcpp.** One target is enough to show value. Adding a second is a day once S3 exists.
- **EVerest's 2.x copy, and OCPP 2.0.1 and 2.1.** These need their own model. The open EVerest bugs are in 2.1, which makes this the most likely next step.
- **Relative profiles, phases, amp and watt conversion, and further proofs.** These extend S2 once the narrow version has found something.
- **The full correspondence review and ambiguity log.** S2's one-page spec and sign-off cover this step.
- **Publishing.** It waits for a result worth publishing.

## Appendix C. Risks

- **The v16 code does not link on its own.** Lands in S1. The kill criterion and fallbacks cover it.
- **EVerest agrees with the model everywhere.** That is a real result, but a weaker one. Lands in S4. The next step is then EVerest's 2.x copy, where the open bugs are.
- **The model is wrong.** A wrong rule produces false fails. S2's sign-off and the cross-check of S1 and S2 on the section 3.13.7 example reduce this. That example has one profile, so it does not test stacking or combining. S3's first fails get read by hand for this reason.
- **Floating point.** EVerest uses floating point. The 0.05 tolerance in S3 covers rounding to one decimal place.
