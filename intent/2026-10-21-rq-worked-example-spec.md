# Spec: A first worked example of a requirement, a spec, assumptions and the proof that links them

Intent: `intent/2026-10-21-rq-worked-example.md`. Each rule is checked by the command under it, run from the repository root.

- **RQE-1.** `formal-verification/requirements/` builds from a clean checkout with one command, and needs no package beyond the Lean toolchain in its `lean-toolchain`.
  `cd formal-verification/requirements && lake build`
- **RQE-2.** `Requirements/PriceAmount.lean` defines `Requirement` (world terms), `Spec` with its clauses `SpecPrecision` and `SpecRange` (the code boundary), and the assumptions `StoredWasWritten`, `CurrencyHasCents` and `ProviderLimit`. `requirement_holds` proves that `Spec` and the three assumptions give `Requirement`.
- **RQE-3.** No theorem listed in `Axioms.lean` depends on `sorry` or on any axiom other than `propext`, `Classical.choice` and `Quot.sound`.
  `formal-verification/requirements/check.sh`
- **RQE-4.** A copy of the file in which A2 says the currency's exponent is 0 fails to build inside `requirement_holds`, and so does a copy in which S1 allows three decimal places. These are two mutations that each break the proof, not a claim that no other clause does.
  `formal-verification/requirements/check.sh`
- **RQE-5.** For each of the five premises of `requirement_holds`, a theorem shows a world where the other four hold and `Requirement` fails.
  `formal-verification/requirements/check.sh` (the `*_needed` theorems build, and their axioms are listed)
- **RQE-6.** `formal-verification/requirements/README.md` traces the range contract on `Price.amount` in `cgv/test_fixtures/tb_max_digits/models.py`, through the edge `record_largest -> Price.amount`, to `SpecRange` and on to `Requirement`. It names the strength of each link as proved, proved relative to the trusted base, observed, or judged, and gives the command that reruns CGV. That command, with `--format text`, exits 0 and prints `0 unverified`. The README says that `SpecPrecision` and `SpecRange` are two of the three contracts CGV extracts from the field, and that the third (non-null) is not modelled.
  `cd cgv && ./target/release/crosscheck-contracts contracts check test_fixtures/tb_max_digits/ --exclude bad.py --lean-checker ./prover/.lake/build/bin/contract-graph-checker --format text`
- **RQE-7.** The branch changes no protected path, and of the rows on the base branch it changes only RQ-1.1, which it sets to `done` with this intent as its record. It adds `todo` rows RQ-1.2 and RQ-1.3.
  `BASE_REF=main PR_BODY='Task: RQ-1.1' node scripts/ci/task-queue.mjs check`

## Concerns flagged
- The link from CGV's contracts to `Spec` is judged, not proved. A wrong reading of CGV's range contract would leave the proof valid and the trace wrong. The four `example`s in the file check the clauses on four fixture values only. RQ-1.2 is the fix.
- `check.sh` locates `requirement_holds` by text. If the theorem moves past the next `/-!` heading, the check of where the error falls needs updating.
- No assumption is observed. The README says how each would be, and that this example has no deployment to observe.
