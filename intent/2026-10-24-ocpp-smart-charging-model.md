# Intent: A verified model of OCPP 1.6 smart charging, and a conformance harness built on it

## Problem statement

OCPP lets a central system tell a charger how much current or power it may draw, by sending charging profiles. The charger combines the profiles it holds into a composite schedule and must stay at or below it. When the combining logic or the profile lifecycle is wrong, a charger can breach a site's supply limit or ignore a grid turn-down.

- Nobody can check a charger's smart charging against every input today. Open Charge Alliance certification runs a fixed set of test cases (OCTT) against each charger.
- The open-source charger stacks have a steady record of smart-charging bugs. EVerest has merged roughly 15 to 20 smart-charging fixes since 2024, and two reports are still open: `EVerest/EVerest#3229` (a 2.1 power limit applied at one third) and `EVerest/EVerest#1060` (the maximum exceeded during 3 to 1 phase switching).
- EVerest keeps two separate copies of the composite calculation, one for OCPP 1.6 (`lib/everest/ocpp/lib/ocpp/v16/profile.cpp`) and one for 2.x (`.../v2/profile.cpp`). A fix to one copy does not reach the other.
- A search found no published formal model of OCPP. The nearest work is testing, not proof (CheckOCPP, OCPPuzz).

## Proposed outcome

Once the first stage lands, two things are true.

- **A machine-checked statement of the core rules.** The OCPP 1.6 edition 2 composite schedule (section 3.13) is a runnable Lean model with two proved properties.
- **EVerest is checked against it.** A differential tester compares EVerest's 1.6 composite calculation with the model on generated profile sets, and every disagreement is triaged as a bug, a spec ambiguity or a model error.

Later stages follow only if the first finds something worth the cost. They are a Quint model of the profile lifecycle, MicroOcpp and EVerest's 2.x code as further targets, and a black-box harness that tests any charger over OCPP.

The honest claim after the first stage is "a proved model of the OCPP 1.6 composite schedule, and what it found in EVerest". It is not "a formally verified OCPP library".

## Affected users and systems

- **EVerest maintainers and the developers of chargers built on EVerest**, who get bug reports with a reproducing profile set.
- **Crosscheck.** The work adds files under `formal-verification/` and exercises `/informal-spec`, `/lean-spec`, `/lean-impl` and `/drt-oracle` on a real external code base.

## Constraints

- **No guarantee rests on an LLM** (`docs/VISION.md`). Claude may draft specs, models and proofs. Only the Lean kernel, the differential tests and human review count as evidence.
- **The composite schedule is an upper bound.** Spec section 3.13.3 says the merged schedule "SHALL be less than or equal to" the lowest limit. Checks fail only when an implementation is above the model, never on inequality alone.
- **No operator data in this repository.** The repository is public. Field data from any charger fleet stays out of it.

## Open questions

- **Where the code lives.** Default: under `formal-verification/` in this repository, following its existing `specs/`, `lean/`, `correspondence/` and `tests/` layout. Move it to its own repository if the harness grows its own users.
- **Errata after edition 2.** Edition 2 (2017-09-28) merges all errata up to v3.0. Whether the Open Charge Alliance published later 1.6 errata is unchecked.
- **OCPP 2.0.1 and 2.1.** Not yet reached. Blocker: the profile structure differs from 1.6, and EVerest keeps a separate v2 copy of the calculation, so the 1.6 model does not cover it. Open question: whether the 1.6 model's shape extends to 2.x. The work follows once the 1.6 harness works, against that v2 copy.
