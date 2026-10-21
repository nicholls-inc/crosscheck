# Intent: A first worked example of a requirement, a spec, assumptions and the proof that links them

Governing roadmap item: RQ-1. Task: RQ-1.1. Spec: `intent/2026-10-21-rq-worked-example-spec.md`.

## Authority
The maintainer granted full autonomy for this run in chat on 2026-10-21 and is away. Open questions are decided below with the simplest option that satisfies the row.

## Problem statement
- `docs/VISION.md` says a spec, together with stated assumptions about the world, can be proved to achieve a requirement once the requirement is written formally. Nothing in the repository does this yet.
- CGV checks that code agrees with the contracts declared on fields. Those contracts do not trace to any requirement (`docs/VISION.md`, "Where the tools stand").
- Later RQ-1 work has no example of the shape to generalise from: which parts are proved, which are observed, and which are judged.

## Proposed outcome
- `formal-verification/requirements/` is a Lake package with no dependencies. `lake build` there checks a Lean file that states:
  - a requirement in world terms;
  - the spec of a code boundary, as the contracts CGV extracts from one field of a fixture under `cgv/test_fixtures/`;
  - the assumptions about the world, each named, and each marked as unprovable by design;
  - a theorem with no `sorry` that the spec and the assumptions give the requirement.
- A script beside it lists the axioms of each theorem and shows that a broken assumption, and a broken spec clause, each make the proof fail.
- A README traces one CGV contract on that fixture to the spec clause and on to the requirement. It gives the command that reruns CGV, and the strength of each link.
- `docs/TASKS.md` marks RQ-1.1 done and adds a row for each generalisation found.

## Affected users and systems
- Readers of `docs/VISION.md` and roadmap item RQ-1, who get one concrete instance of the chain.
- Authors of later RQ-1 rows.
- No tool changes. CGV, the Crosscheck plugin and CI are untouched.

## Constraints
- No protected surface changes. CGV's `BehaviorModel.lean`, its protected statements and its fixtures stay as they are.
- The Lean proofs use no axiom other than `propext`, `Classical.choice` and `Quot.sound`.
- Each link of the trace names its strength in the vision's words: proved, proved relative to the trusted base, observed, or judged. No link is called "verified".
- Only the RQ-1.1 row of `docs/TASKS.md` changes, apart from new `todo` rows.

## Decided without the maintainer
- **Which requirement.** A stored price is a whole number of the currency's minor units, and the payment provider does not refuse a charge or refund of it for its size. The field is `Price.amount` in `cgv/test_fixtures/tb_max_digits/`, `DecimalField(max_digits=5, decimal_places=2)`. CGV extracts three contracts from it (a precision bound, a range and an implicit non-null), of which the spec models the first two as its two clauses and leaves non-null unmodelled, and the fixture's `ok.py` and `bad.py` already show CGV accepting and rejecting writes. An email length requirement was the other candidate. It would have had one clause and a weaker assumption about the world.
- **Notation.** Plain Lean 4 with no Mathlib, so the build needs no network and takes seconds. A Python `Decimal` is modelled as an integer coefficient and a count of decimal places. Controlled English (CE-1) is not reached yet, so the README states each clause in plain English beside its Lean name.
- **Where it lives.** A new package, `formal-verification/requirements/`. `formal-verification/lean/` has no Lake file, and its one module imports Mathlib and contains `sorry` by design, as a pipeline stub.
- **The strength of the link from CGV's contracts to the Lean spec.** Judged. The spec was written by reading CGV's extracted contracts, and no theorem relates it to CGV's Lean definitions. Making that link proved is a new row, RQ-1.2.
- **CI.** Not added. Workflows are a protected surface and the row does not ask for it. Running `check.sh` in CI is a new row, RQ-1.3.
- **Tier.** Tier 2. The change adds a buildable Lean package, so it carries a spec, though no existing behaviour changes.

## Open questions
None.
