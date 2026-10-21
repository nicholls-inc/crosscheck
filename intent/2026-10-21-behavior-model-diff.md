# Intent: Test `BehaviorModel.lean` against Django and pydantic

Task: TB-1.14. Governing roadmap item: TB-1. Issue: #16 (approach D for link 3).
Plan: step 4 of `intent/2026-10-07-prove-extraction-plan.md`.
Spec: `intent/2026-10-21-behavior-model-diff-spec.md`.

## Problem statement
`cgv/prover/ContractGraph/BehaviorModel.lean` states what Django 4.2 and 5.x fields and pydantic 2 validation accept, in 15 predicates. It is in CGV's trusted base, and nothing checks it against the libraries it describes. Its one known error was found by hand: CG-1.17 records that `pydanticDecimalAccepts` rejects `Decimal("1.2300")` for `Field(decimal_places=2)`, which pydantic 2.13.5 accepts. Nothing in `cgv/scripts/` compares the predicates with the libraries for any other input (measured, grep for `BehaviorModel` in `cgv/scripts/` on `origin/main` at `2efb07c`).

## Proposed outcome
A rerunnable harness compares every predicate with the real library, on inputs that Hypothesis draws from a fixed seed. For each predicate it runs the Django model field's `clean` (Django 4.2.30 and 5.2.18, each on two database backends) or pydantic 2.13.5 validation, and compares the verdict with the predicate evaluated by a Lean program that imports the `BehaviorModel.lean` text. It reports, per configuration and predicate, how many inputs it drew and how many disagree, with examples. A committed baseline holds, under the default seed, the inputs drawn and the disagreements in each direction, and the harness fails when a count differs from it. A self-test mutates one predicate of each family in a scratch copy and shows that the harness fails on each.

Each disagreement is a finding about the model. This pull request records each one and adds a `todo` row for any that no row covers. It does not amend `BehaviorModel.lean`, which is a protected surface. The claim the harness supports is `tested`, with its count and seed. Running it in CI is TB-1.24.

## Affected users and systems
- Maintainers of CGV's trust model. They get a count of how often each trusted predicate agrees with the library, and a list of where it does not.
- TB-1.22 reuses the Lean evaluator, so there is one evaluator of the Lean text and no Python copy of it.
- TB-1.24 runs the harness in CI.
- New files: `cgv/scripts/behavior_diff.py`, `cgv/scripts/behavior-requirements-django42.txt`, `cgv/scripts/behavior-requirements-django52.txt`, `cgv/scripts/behavior-baseline.json`, `cgv/prover/scripts/BehaviorEval.lean`, and a results record under `cgv/docs/evaluation/`. `docs/TASKS.md` gets the TB-1.14 row set to `done` and the new rows.

## Constraints
- `BehaviorModel.lean` does not change. No protected surface changes.
- The predicates are evaluated by Lean from the committed text, never by a Python copy of their bodies.
- Everything reruns: pinned packages with hashes, a pinned Python minor version, the pinned Lean toolchain, and a fixed seed (vision rule 3).
- No LLM verdict is evidence (vision rule 1). The harness is deterministic.

## Open questions
None. The decisions below were open, and the coordinating agent took them without consulting the maintainer on each one.

## Decided without the maintainer
The coordinating agent decided each question below under the full-autonomy and merge grant the maintainer gave it directly in chat on 2026-10-21. This file records that authorization. It does not constitute it, and nothing in the repository grants it. Each open question was decided with the most conservative option. Decisions 5 and 6 were revised in the review of #134 under the same authorization.

1. **How digits are counted.** The predicates take digit counts, and the Lean text does not say how a decimal value gives them. The harness reads them from the decimal as written: fractional digits are the places after the point, including trailing zeros; integer digits are the digits of the integer part, so zero has none; total digits are their sum. This is the plain reading of "v has at most d fractional digits". Where Django or pydantic count differently, the disagreement is reported, not absorbed into the reading. The alternative, copying each library's counting into the harness, would make the comparison agree by construction.
2. **One field option at a time.** Each Django field sets only the option its predicate describes and `blank=True`, so `blank` does not reject `""` for a predicate that is not about `blank`. `choicesAccepts` is the exception: Django skips the choices check for an empty value and lets `blank` decide, so the harness draws `blank` as an input and reports both.
3. **Database backends.** Django's integer fields take their bounds from the database backend. The harness runs the Django predicates on SQLite and on Django's base ranges (the ones PostgreSQL uses), both without a database server. MySQL and Oracle have their own ranges and are not yet reached.
4. **Two requirement files.** The plan named one `behavior-requirements.txt`. Django 4.2 and 5.2 cannot be installed together, so there is one file per Django version. pydantic and Hypothesis are pinned to the same versions in both.
5. **Known disagreements do not fail the run.** A harness that fails on every disagreement fails today, so a mutant would not change its verdict. The baseline records, per configuration and predicate under the default seed and example count, the inputs drawn and the disagreements in each direction: the library accepts and the model rejects, and the model accepts and the library rejects. The harness fails when any of these counts differs. A total alone would let a change that adds disagreements in one direction and removes as many in the other pass. A known disagreement therefore stays visible in every report and in the baseline, and each one names the task row that tracks it.
6. **Draws do not depend on the harness text.** Hypothesis 6.170.5 mixes string and number literals from local source files, `behavior_diff.py` among them, into its draws. Editing the harness then changed the inputs: adding the self-test mutants moved `annotationAcceptsNumeric` from 66 to 68 disagreements with nothing else changed. The harness turns that source off and keeps Hypothesis's built-in constants. The inputs changed once, and the counts in the evaluation record are from the new draws. The same five predicates disagree, for the same five reasons.
