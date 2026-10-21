# Requirements: a spec and assumptions that achieve a requirement

This package is the first worked example of roadmap item RQ-1 (task RQ-1.1). It covers the second link of the chain in [`docs/VISION.md`](../../docs/VISION.md), goal → requirement → spec → code. A spec, together with stated assumptions about the world, is proved to achieve a requirement written formally. A trace then ties one CGV contract on a fixture to that spec and on to the requirement.

The example is kept small so that its shape is easy to see. Later RQ rows generalise it.

## Rerun

From a clean checkout, with [elan](https://github.com/leanprover/elan) installed:

```bash
cd formal-verification/requirements && lake build
```

`lake build` uses the toolchain in `lean-toolchain` (Lean v4.28.0, as `cgv/prover` uses). The package has no dependencies, so the build needs no network after elan has fetched the toolchain.

The full check also lists the axioms of every theorem and breaks two premises on purpose:

```bash
formal-verification/requirements/check.sh
```

It fails if any theorem uses an axiom other than `propext`, `Classical.choice` and `Quot.sound` (so a `sorry` fails it). It then builds two broken copies of `Requirements/PriceAmount.lean` in a temporary directory. In one, A2 says the currency has no minor unit. In the other, S1 allows three decimal places. Each copy must fail inside `requirement_holds`:

```text
'Requirements.PriceAmount.requirement_holds' depends on axioms: [propext, Quot.sound]
...
ok: only propext, Classical.choice and Quot.sound
ok: A2 with a currency of exponent 0 breaks requirement_holds:
error: Requirements/PriceAmount.lean:102:28: omega could not prove the goal:
ok: S1 with three decimal places breaks requirement_holds:
error: Requirements/PriceAmount.lean:101:15: omega could not prove the goal:
all RQ-1.1 checks passed
```

## The example

The goal in the world is that a customer can be charged, or refunded, the price the system holds. The code is the fixture [`cgv/test_fixtures/tb_max_digits/`](../../cgv/test_fixtures/tb_max_digits/), where `Price.amount` is `DecimalField(max_digits=5, decimal_places=2)`.

### 1. The requirement, in world terms

> **R.** Every stored price is a whole number *n* of the currency's minor units, and the payment provider does not refuse a charge or refund of *n* minor units for its size.

In Lean this is `Requirement`. It speaks of the stored prices, the currency, and the provider. It does not mention the code.

### 2. The spec of the code boundary

> **S1.** Every amount the application writes to `Price.amount` has at most two decimal places.
>
> **S2.** Every amount the application writes to `Price.amount` is between -999.99 and 999.99.

In Lean these are `SpecPrecision`, `SpecRange` and `Spec`. They are the two contracts that CGV extracts from the field.

### 3. The assumptions about the world

None of these can be proved, by design. Each is a fact about the world, so it is observed or not. In this example none is observed yet, because the fixture has no deployment.

| Name | Assumption | How it would be observed |
| --- | --- | --- |
| A1 `StoredWasWritten` | Every stored amount reached the column through a write on a data path that CGV checks, and the database keeps it exactly. No raw SQL, migration, admin edit or second application writes the column. | Database grants that let only the application write the table, and a periodic query that every row meets S1 and S2. |
| A2 `CurrencyHasCents` | The currency's minor unit is a hundredth of its major unit, as for GBP, EUR and USD. | The configured currency code, and its exponent in ISO 4217. |
| A3 `ProviderLimit` | The payment provider does not refuse for its size any charge or refund of at most 99,999,999 minor units. | The provider's published limit, and an acceptance test in its sandbox at that limit. |

### 4. The proof

`requirement_holds` proves `Spec w → StoredWasWritten w → CurrencyHasCents w → ProviderLimit w → Requirement w` for every world `w`. It has no `sorry`, and it uses only `propext` and `Quot.sound`.

Five more theorems (`storedWasWritten_needed`, `currencyHasCents_needed`, `providerLimit_needed`, `specPrecision_needed`, `specRange_needed`) show that each premise is needed. For each one, they build a world where the other four hold and R fails. For example, with a currency whose exponent is 0, as for JPY, a stored `0.01` is not a whole number of minor units.

### 5. The trace from a CGV contract to the requirement

The contract is the range contract on `Price.amount`, from `models.py:8`, and the edge is `record_largest -> Price.amount`, the write `Price.objects.create(amount=Decimal("999.99"))` at `ok.py:8`. The source guarantees `range ≤ 999.99` and the target requires `range ≤ 999.99`, so the bound is met exactly. The precision contract on the same field (`decimal_places=2`) traces to S1 in the same way.

Rerun CGV on it from `cgv/`:

```bash
cargo build --release && (cd prover && lake build)
./target/release/crosscheck-contracts contracts check test_fixtures/tb_max_digits/ \
  --exclude bad.py --lean-checker ./prover/.lake/build/bin/contract-graph-checker
```

This exits 0. Without `--exclude bad.py`, it exits 1 and reports three range errors. Two of them are on `Price.amount` (`123456.78` and `-1000`). Those writes break S2, so for the full fixture `Spec` does not hold and this proof says nothing about the requirement. The `example`s at the end of `PriceAmount.lean` evaluate S1 and S2 on the fixture's values, and they agree with CGV's verdicts.

| Link | From → to | Evidence | Strength |
| --- | --- | --- | --- |
| 1 | The code → CGV's contracts hold on every checked data path | CGV exit 0 on the command above, and `runChecker_sound_all` in `cgv/prover/ContractGraph/Main.lean` | Proved, relative to the trusted base: the Lean kernel, the Rust extractor, `Translation.lean` and `BehaviorModel.lean`. Extraction is not proved (issue #16). |
| 2 | CGV's contracts → `Spec` (S1 and S2) | The two clauses were written by reading the contracts in CGV's database: `precision` with `decimal_places = 2`, and `range` from -999.99 to 999.99. The four `example`s check them on four fixture values. | Judged. No theorem relates `Spec` to CGV's Lean definitions. RQ-1.2 makes this link proved. |
| 3 | `Spec` and A1 to A3 → R | `requirement_holds`, rerun by `lake build` | Proved, relative to the assumptions and the Lean kernel |
| 4 | A1 to A3 are true of the world | The observations in the table above | Observed, and not yet observed for this example |
| 5 | R achieves the goal | A person who knows payments reads R | Judged, and not yet judged |

## What this example does not cover

- The provider also refuses charges below a minimum. R does not say that a stored price is above the minimum, because the field contract does not serve that. It would be a second requirement, served by a different spec.
- On PostgreSQL, the column type `numeric(5, 2)` also bounds what is stored. That is a second route to R, through an assumption about the database instead of a spec of the code. This example takes the route through the code, because that route is the one CGV checks.
- Nothing in CI reruns this package yet. RQ-1.3 adds it.
