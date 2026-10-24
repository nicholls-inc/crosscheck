# Intent: A verified model of OCPP 1.6 smart charging, and a conformance harness built on it

## Problem statement

OCPP lets a central system tell a charger how much current or power it may draw, by sending charging profiles. The charger combines the profiles it holds into a composite schedule and must stay at or below it. When the combining logic or the profile lifecycle is wrong, a charger can breach a site's supply limit or ignore a grid turn-down.

- Nobody can check a charger's smart charging against every input today. Open Charge Alliance certification runs a fixed set of test cases (OCTT) against each charger.
- The open-source charger stacks have a steady record of smart-charging bugs. EVerest has merged roughly 15 to 20 smart-charging fixes since 2024, and two reports are still open: `EVerest/EVerest#3229` (a 2.1 power limit applied at one third) and `EVerest/EVerest#1060` (the maximum exceeded during 3 to 1 phase switching).
- EVerest keeps two separate copies of the composite calculation, one for OCPP 1.6 (`lib/everest/ocpp/lib/ocpp/v16/profile.cpp`) and one for 2.x (`.../v2/profile.cpp`). A fix to one copy does not reach the other.
- A search found no published formal model of OCPP. The nearest work is testing, not proof (CheckOCPP, OCPPuzz).

## Proposed outcome

Once this lands, three things are true.

- **A machine-checked statement of the rules.** OCPP 1.6 edition 2, section 3.13, smart charging is written as a Lean model with proved properties, and the charger-side profile lifecycle is written as a Quint model with checked invariants.
- **EVerest and MicroOcpp are checked against it.** A differential tester compares their 1.6 composite calculations with the model on generated profile stacks, and every disagreement is triaged as a bug, a spec ambiguity or a model error.
- **Anyone can test a charger against it.** A black-box harness drives a charger over OCPP with `SetChargingProfile` and `GetCompositeSchedule` and reports where its answers break the rules. It needs no access to the charger's source.

The honest claim is "a verified model of OCPP 1.6 smart charging, and a conformance harness that uses it". It is not "a formally verified OCPP library".

## Affected users and systems

- **Charger and firmware developers**, including EVerest and MicroOcpp maintainers, who get bug reports with a reproducing profile stack.
- **Central system operators and aggregators**, who can check what a charger reports against the profiles they sent.
- **The Open Charge Alliance**, which gets precise reports of ambiguous or contradictory spec text.
- **Crosscheck.** The work adds files under `formal-verification/` and exercises `/informal-spec`, `/lean-spec`, `/lean-impl`, `/correspondence-review` and `/drt-oracle` on a real external code base.

## Constraints

- **No guarantee rests on an LLM** (`docs/VISION.md`). Claude may draft specs, models and proofs. Only the Lean kernel, the Quint checkers, the differential tests and human review count as evidence.
- **The OCPP spec is CC BY-ND 4.0.** Quote it word for word with attribution, or describe it in original words with section numbers. Never publish modified spec text.
- **The composite schedule is an upper bound.** Spec section 3.13.3 says the merged schedule "SHALL be less than or equal to" the lowest limit, and section 5.7 says the reported schedule "is only indicative". Checks test at most, never exact equality.
- **The TxProfile deletion rule is a SHOULD.** Section 3.13.1 says that after a transaction ends the profile "SHOULD be deleted". The lifecycle model accepts chargers that keep it and chargers that delete it.
- **`GetCompositeSchedule` is optional for chargers.** It belongs to the Smart Charging feature profile. Chargers without it get a meter-only check or none.
- **Quint checking is bounded.** Apalache explores behaviours up to a step bound, so Quint results are evidence, not proofs. Unbounded claims come from Lean only.
- **No operator data in this repository.** The repository is public. Field data from any charger fleet stays out of it.

## Open questions

- **Licence reading.** Does a Lean or Quint model that cites OCPP section numbers count as an adaptation under CC BY-ND? Default: treat it as original work, quote nothing modified, and get a legal reading before the first public release.
- **Where the code lives.** Default: under `formal-verification/` in this repository, following its existing `specs/`, `lean/`, `correspondence/` and `tests/` layout. Move it to its own repository if the harness grows its own users.
- **Ownership.** Who owns the work needs confirming in writing before publishing beyond this repository.
- **Errata after edition 2.** Edition 2 (2017-09-28) merges all errata up to v3.0. Whether the Open Charge Alliance published later 1.6 errata is unchecked.
- **OCPP 2.0.1 and 2.1.** Out of this program. They follow once the 1.6 harness works, against EVerest's separate v2 copy.
