# Plan: OCPP 1.6 smart-charging model and conformance harness

Intent: `intent/2026-10-24-ocpp-smart-charging-model.md`. Each phase writes its own `spec.md` at phase start, per `docs/assurance/DEVELOPMENT-FRAMEWORK.md`. No phase touches a protected surface.

The program writes OCPP 1.6 edition 2 smart charging as a Lean model with proved properties and a Quint model of the profile lifecycle. It then checks EVerest's and MicroOcpp's 1.6 composite calculations against the model, and builds a black-box harness that tests any charger over OCPP. Phases run in order, P1 to P8. Each phase is one pull request with its own evidence.

## How to read this

One box is one unit of work. Every box names the evidence that checks it. Check a box only when its evidence exists, a file, a command's output, a report, or a link. The body is a how-to. The appendices explain and record.

Only deterministic checks and human review count as evidence (`docs/VISION.md`). A Lean build with no `sorry` and no extra axioms, a Quint run, a differential test run and a reviewer's sign-off are evidence. An LLM's review is a pointer to look at, never a tick.

This plan departs from the pstack multi-phase skeleton in two places. There is no UI, so the "live" lanes are runs of the model checkers and harnesses. There is no perf gate, except a runtime budget on the harnesses.

Time per phase is a guess for part-time work.

| Phase | Output | Guess |
| --- | --- | --- |
| P0 | Operator items cleared, toolchain installed | 1 week |
| P1 | Informal spec and ambiguity log | 1 week |
| P2 | Lean model and proofs | 2 weeks |
| P3 | Executable model and correspondence review | 1 week |
| P4 | Quint lifecycle model | 2 weeks |
| P5 | Differential tests against EVerest and MicroOcpp | 2 weeks |
| P6 | Black-box harness over OCPP | 2 weeks |
| P7 | Triage and upstream reports | 1 week, then ongoing |
| P8 | Publish | 1 week |

## Clear the operator items (P0)

These need the operator, not an agent.

- [ ] Confirm in writing who owns this work. Evidence is the operator's own record, kept out of this repository.
- [ ] Get a reading on whether a model citing OCPP section numbers is an adaptation under CC BY-ND 4.0. Evidence is a note in `formal-verification/ocpp16-LICENCE-NOTE.md` that says who gave the reading and when, with no legal text copied.
- [ ] Check the Open Charge Alliance download page for OCPP 1.6 errata newer than edition 2 (2017-09-28). Evidence is a line in the P1 spec naming the edition and any errata used.
- [ ] Record the spec source. Note the file's sha256 in the P1 spec and cite it as "OCPP 1.6 edition 2, Open Charge Alliance, 2017-09-28".
- [ ] Install the toolchain and record versions in `formal-verification/tests/ocpp16_composite/README.md`.
  - [ ] Lean through the existing image, `crosscheck/scripts/build-lean-docker.sh`. Evidence is a passing `lean_check` on an empty module.
  - [ ] Quint (`@informalsystems/quint` on npm) and Apalache (needs Java 17). Evidence is `quint --version` and `quint verify` on a one-line spec.
  - [ ] EVerest at a pinned commit, built with its SIL config `config/config-sil-ocpp.yaml`. Evidence is the SIL connecting to a local central system and sending `BootNotification`.
  - [ ] MicroOcpp at a pinned release (v1.2.0 or later). Evidence is its test suite passing.

## Write the informal spec (P1)

**Depends on.** P0.

**Files.**

- [ ] Create `formal-verification/specs/ocpp16_smart_charging_informal.md`.
- [ ] Create `formal-verification/specs/ocpp16_ambiguities.md`.

**Build.**

- [ ] Run `/informal-spec` over sections 3.13, 5.7, 5.16 and the classes 7.13 and 7.14. Each requirement gets an ID (`OC-1`, `OC-2` and on) and the section it comes from. Describe each rule in original words and quote word for word only where the exact wording matters.
- [ ] Cover these rules.
  - [ ] Profile purposes and connector 0 meaning (3.13.1).
  - [ ] Stack levels, and the highest valid level winning within a purpose (3.13.2).
  - [ ] Combining purposes, with the merged schedule at or below the lowest limit (3.13.3).
  - [ ] Profile kinds Absolute, Recurring and Relative, and validity windows.
  - [ ] Units A and W, `numberPhases` defaulting to 3 (7.14), and limits with at most one decimal place (7.13).
  - [ ] TxProfile rejected with no active transaction, and applying "for the duration of the current transaction only" (3.13.1).
  - [ ] `GetCompositeSchedule` replies being "only indicative" (5.7).
  - [ ] Discovery of charger capabilities (3.13.5) and offline behaviour (3.13.6).
- [ ] Log every rule that has two readings in `ocpp16_ambiguities.md`, with both readings and the section.

**You see.**

- [ ] A requirement list where every ID cites a section, and an ambiguity log with at least the TxProfile SHOULD-delete case.

**Verify.**

- [ ] The operator checks each requirement against the PDF and signs the file's review table. `/spec-adversary` may run first, and its findings are pointers only.

## Prove the composite rules in Lean (P2)

**Depends on.** P1.

**Files.**

- [ ] Create `formal-verification/lean/CrosscheckModel/Ocpp16/Profile.lean`.
- [ ] Create `formal-verification/lean/CrosscheckModel/Ocpp16/Composite.lean`.

**Build.**

- [ ] Name the data shape first, with `/lean-spec`.
  - [ ] `Limit` is a natural number of tenths, because the spec allows one decimal place. No floats.
  - [ ] `RateUnit` is `amps` or `watts`. `Purpose` is `chargePointMax`, `txDefault` or `tx`.
  - [ ] Time is whole seconds. A schedule is a start, an optional duration and a list of periods.
  - [ ] `composite : ProfileSet → Connector → Time → Option Limit`.
- [ ] Prove these properties.
  - [ ] T1. Where a valid ChargePointMaxProfile covers time t, the composite at t is at or below its limit.
  - [ ] T2. The composite at t is at or below the winning TxProfile or TxDefaultProfile limit at t.
  - [ ] T3. Where a valid TxProfile covers t, no TxDefaultProfile affects the composite at t.
  - [ ] T4. Within one purpose, only the highest valid stack level at t affects the composite.
  - [ ] T5. Adding or tightening a ChargePointMaxProfile never raises the composite at any t.
  - [ ] T6. Converting between A and W under a stated voltage and phase count never loosens a limit.

**You see.**

- [ ] `lake build` passes and the theorem list prints T1 to T6.

**Verify.**

- [ ] `lean_check` passes on both files.
- [ ] Create `formal-verification/lean/CrosscheckModel/Ocpp16/check.sh`, modelled on `formal-verification/requirements/check.sh`. It prints the axioms of T1 to T6 and fails on anything beyond `propext`, `Classical.choice` and `Quot.sound`, which also catches `sorry`.
- [ ] The operator reads each theorem statement against its requirement ID. A proof is only as good as its statement.

## Make the model executable and review the correspondence (P3)

**Depends on.** P2.

**Files.**

- [ ] Create `formal-verification/correspondence/Ocpp16Composite.md`.

**Build.**

- [ ] Run `/lean-impl` so `composite` runs, and smoke-test it with `lean_run` on the use cases in 3.13.4 and the example data structure in 3.13.7.
- [ ] Run `/correspondence-review` from requirement IDs to Lean definitions and theorems.

**You see.**

- [ ] The model's output on each example in 3.13.4 and 3.13.7 matches what the spec describes.

**Verify.**

- [ ] `lean_run` output for those examples is saved in the correspondence file.
- [ ] The operator signs the correspondence table.

## Model the profile lifecycle in Quint (P4)

**Depends on.** P1. It can run alongside P2 and P3.

**Files.**

- [ ] Create `formal-verification/quint/ocpp16_profiles.qnt`.
- [ ] Create `formal-verification/quint/README.md`.

**Build.**

- [ ] State per connector is the transaction (none or an ID), the stored profiles and whether the charger is online.
- [ ] Actions are `SetChargingProfile`, `ClearChargingProfile`, `StartTransaction`, `StopTransaction`, `Reboot`, going offline, coming back, and time passing.
- [ ] Model the SHOULD in 3.13.1 as a nondeterministic choice, delete or keep, so both kinds of charger are allowed.
- [ ] Let the central system reuse transaction IDs, because 1.6 does not require them to be unique.
- [ ] Check whether 1.6 requires stored profiles to survive a reboot. Model what the spec says and log any gap in the ambiguity log.
- [ ] Write the invariants.
  - [ ] I1. A TxProfile is accepted only while a transaction is active on its connector.
  - [ ] I2. A TxProfile set during one transaction never affects a later transaction, even when it was kept and the ID was reused.
  - [ ] I3. With a valid ChargePointMaxProfile stored, the limit in force never exceeds it.

**You see.**

- [ ] `quint verify` reports no violation up to the stated bound. A deliberately broken variant, one that applies a kept TxProfile to the next transaction, produces a counterexample trace.

**Verify.**

- [ ] `quint typecheck`, `quint test` and `quint run --max-samples=10000` pass.
- [ ] `quint verify --max-steps=12` passes, and the bound is written in the README.
- [ ] The broken variant's counterexample is saved as an ITF trace.

## Test EVerest and MicroOcpp against the model (P5)

**Depends on.** P3.

**Files.**

- [ ] Create `formal-verification/tests/ocpp16_composite/drt_harness.py`, `README.md` and `drt_report.md`.
- [ ] Create `formal-verification/tests/ocpp16_composite/everest_driver/`, a small C++ program that links EVerest's `lib/everest/ocpp/lib/ocpp/v16/profile.cpp` at the pinned commit.
- [ ] Create `formal-verification/tests/ocpp16_composite/microocpp_driver/`, the same for `SmartChargingService.cpp`.

**Build.**

- [ ] Run `/drt-oracle` with the Lean model as the oracle.
- [ ] Generate profile stacks with up to four profiles per purpose and up to five periods each, in both units and all three kinds.
- [ ] Classify each case.
  - [ ] Fail when the implementation's limit is above the model's.
  - [ ] Warn when it is below the model's by more than 0.05. That is allowed by the spec but worth a look.
  - [ ] Pass otherwise.
- [ ] Record the seed, the case count and both pinned versions in `drt_report.md`.

**You see.**

- [ ] `drt_report.md` lists counts of pass, warn and fail per implementation, with a minimal reproducing stack for every fail.

**Verify.**

- [ ] 10,000 cases per implementation run in under 10 minutes on a laptop.
- [ ] A mutated driver, one with the stack-level comparison reversed, produces fails. This shows the harness can catch a bug.

## Build the black-box harness over OCPP (P6)

**Depends on.** P4 and P5.

**Files.**

- [ ] Create `formal-verification/tests/ocpp16_blackbox/`, with a small central system using `mobilityhouse/ocpp`, a scenario runner that reads Quint ITF traces, the checker and a `docker-compose.yml` for the EVerest SIL.

**Build.**

- [ ] Replay each trace's messages against the charger under test.
- [ ] Check each step.
  - [ ] The charger accepts or rejects as the model allows.
  - [ ] Each `GetCompositeSchedule` reply is at or below the model's limit at every instant.
  - [ ] Where the charger sends `MeterValues`, current or power stays at or below the reported limit after a stated settling time.
- [ ] Mark a charger that does not support `GetCompositeSchedule` as "composite not testable" and run the meter check only.

**You see.**

- [ ] One command runs every trace against the EVerest SIL and prints a per-trace verdict.

**Verify.**

- [ ] The harness runs end to end against the EVerest SIL and MicroOcpp's simulator.
- [ ] A fault-injected central system, one that sends a TxProfile with no transaction, produces the expected rejection check.

## Triage and report upstream (P7)

**Depends on.** P5 and P6.

- [ ] Classify every fail and warn as an implementation bug, a spec ambiguity or a model error. A model error goes back to P2 or P4 and the runs repeat.
- [ ] For each implementation bug, write a minimal reproducer and a draft issue. The operator files it. Evidence is the issue link in `drt_report.md`.
- [ ] For each ambiguity, draft a report for the Open Charge Alliance. The operator sends it.

## Publish (P8)

**Depends on.** P7.

- [ ] Write `formal-verification/OCPP16.md` covering what is proved, what is checked to a bound, what is only tested, and what is not yet reached.
- [ ] Use only the claim "a verified model of OCPP 1.6 smart charging, and a conformance harness that uses it".
- [ ] Describe the method, including what Claude drafted, what the Lean kernel and Quint rejected, and what the operator changed.
- [ ] Add the CC BY-ND attribution for every verbatim quote.

## Close the program

- [ ] Every box above is checked with its evidence.
- [ ] A journal entry in `JOURNAL.md` records the outcome and the open follow-ups.

## Appendix A. Prototype evidence, and the facts this plan relies on

No prototype was run. The facts below were checked against sources instead.

Each was checked against a primary source on 2026-10-24.

- The OCPP 1.6 edition 2 rules quoted above are in the edition 2 FINAL PDF of 2017-09-28. Sections 3.13.1, 3.13.3, 5.7, 7.13 and 7.14 were matched word for word, and the section numbers 3.3, 3.13.4 to 3.13.7 and 5.16 were read from the contents page.
- The spec is CC BY-ND 4.0, per the licence notice on its opening pages.
- `GetCompositeSchedule` is in the Smart Charging feature profile, which is optional for chargers (spec 3.3, and the OCA certification page).
- EVerest's OCPP code is in `EVerest/EVerest` under `lib/everest/ocpp/`. `EVerest/libocpp` is archived. The v16 and v2 profile code are separate copies (`v16/profile.cpp`, 719 lines, and `v2/profile.cpp`, 1,253 lines, each with its own `merge_profiles_by_lowest_limit`).
- EVerest has SIL configs that run without hardware against a configurable central system (`config/config-sil-ocpp.yaml`).
- MicroOcpp has 1.6 composite logic in `src/MicroOcpp/Model/SmartCharging/SmartChargingService.cpp`. Its 2.0.1 support is alpha and has no smart charging.
- Apalache is a bounded model checker, and Quint exports ITF traces. Quint is now developed by Quint Co (`quint-co/quint`).
- OCA certification is test-based (OCTT and a PICS).
- No formal model of OCPP was found. The search was not exhaustive, so recheck DBLP by hand before claiming novelty.

Still unproven. No prototype has built the EVerest v16 driver or the SIL setup, so the P0 and P5 estimates are guesses.

## Appendix B. Alternatives rejected

- **A verified replacement library now.** Lean-compiled C needs the Lean runtime, so it is not a drop-in for EVerest. F\* with Low\* emits plain C and is the route if a replacement is wanted later.
- **Bend 2.** It has dependent types and emits C, but it was released in September 2026 and its notes say the compiler is "99% AI-written and not yet fully audited".
- **Starting with OCPP 2.0.1 or 2.1.** The 1.6 spec is smaller and MicroOcpp has 1.6 smart charging only. 2.x follows against EVerest's separate v2 copy.
- **Exact-match checking.** The spec makes the composite an upper bound and the reply indicative, so exact matching would fail compliant chargers.
- **TLA+ instead of Quint.** Both fit. Quint has the same checkers behind a typed syntax and exports traces the harness can replay.

## Appendix C. Risks

- **Licence.** A reading that models count as adaptations would block publishing. Lands in P0. Watch for the reading before P8.
- **Ownership.** Unresolved ownership blocks publishing. Lands in P0.
- **EVerest build cost.** Linking `v16/profile.cpp` alone may pull in much of the tree. Lands in P5. Fall back to driving EVerest through its SIL only.
- **Floating point.** EVerest and MicroOcpp use floats. The 0.05 tolerance in P5 covers rounding to one decimal place. Watch for fails that vanish with a wider tolerance.
- **Bounded checking.** Quint finds no bug beyond its bound. The README states the bound, and unbounded claims come only from Lean.
- **Spec misreading.** A wrong requirement gives a proof of the wrong thing. P1 and P3 have operator sign-off for this reason.

## Appendix D. Reading list

- OCPP 1.6 edition 2, sections 3.13, 5.7, 5.16, 7.13 and 7.14.
- `docs/VISION.md` and `docs/assurance/DEVELOPMENT-FRAMEWORK.md`.
- The `/informal-spec`, `/lean-spec`, `/lean-impl`, `/correspondence-review` and `/drt-oracle` skills in `crosscheck/skills/`.
- The pipeline's existing worked example, `formal-verification/specs/power_informal.md`, `formal-verification/lean/CrosscheckModel/Power.lean`, `formal-verification/correspondence/Power.md` and `formal-verification/tests/power/`.
- EVerest `lib/everest/ocpp/lib/ocpp/v16/smart_charging.cpp` and `v16/profile.cpp`. MicroOcpp `SmartChargingService.cpp`.
