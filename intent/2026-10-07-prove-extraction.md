# Intent: Plan the proof of CGV's extraction, and split it into rows

Task: TB-1.4. Governing roadmap item: TB-1. Issue: #16.
Plan: `intent/2026-10-07-prove-extraction-plan.md`.

## Problem statement
`runChecker_sound_all` (`cgv/prover/ContractGraph/Main.lean:806`) proves that exit 0 means every data path of the graph handed to `runChecker` is `stepwiseSound`. Three unproved links produce that graph, so exit 0 says the checker found no inconsistency in what the extractor claimed. It does not say the Python code is consistent.

1. **Source to rows.** The Rust extractor (`cgv/src/`, 19 files, 16,715 lines on `origin/main` at `88e7b33`, measured with `wc -l`) turns Python into SQLite rows. It is untrusted and tags each fact `[EXTRACTED]`.
2. **Rows to graph.** `cgv/prover/ContractGraph/Translation.lean` (623 lines, no theorems) turns the rows into a `ContractGraph`.
3. **Graph to behaviour.** `BehaviorModel.lean` states what Django and pydantic accept. No theorem mentions it, so `constraintImplies` is not linked to any acceptance predicate.

Issue #16 proposes four approaches: (A) prove `Translation.lean`, (B) re-check extractor certificates in Lean, (C) link `constraintImplies` to `BehaviorModel.lean`, and (D) differential tests against real Django and pydantic. It recommends C, then A, with D in parallel, and B piloted on one pattern after D. Nothing in the queue does any of this yet. Row TB-1.4 asks for the intent, the plan and the rows.

Reading the code for this intent found four facts that change the rows.

- **`max_digits` is never checked (measured).** The extractor writes `param_max_digits`, and `readContracts` selects the column, but `ContractRow` has no field for it, so the value is dropped. `decimalFieldAccepts` has three conjuncts, and the checker compares only decimal places. A fixture with `DecimalField(max_digits=5, decimal_places=2)` and a write of `Decimal('123456.78')` exits with no finding for that write. The same run reports `Decimal('1.234')` in the other function. The run used an extractor and a checker built from earlier task branches. The code that drops the column is the same on `origin/main` (read from code). That is a missed bug, so a link-3 theorem for `precision` can state only the fractional-digit conjunct until the check exists.
- **Translation drops rows it cannot place (read from code).** `cgv/src/db.rs` opens the database with `PRAGMA foreign_keys = OFF`. `buildGraph` drops an edge whose endpoint id names no node (`filterMap`), and ignores a contract row whose `node_id` names no node. Neither reaches exit 2. The trust table in `cgv/README.md` says translation "rejects malformed rows (exit 2) rather than dropping them". That holds for the three malformed-value checks in `ContractRow.malformed`, not for these rows.
- **Four parsers fall back silently (read from code).** `parseConstraintKind`, `parseVerifLevel`, `parseContractRole` and `parseRelationship` map an unknown string to a default. Only the SQLite `CHECK` constraints written by `cgv/src/db.rs` keep such strings out, and Lean does not know that.
- **The numeric tower is already queued.** CG-1.7 adds `int` into `float` to `constraintImplies` and a rule to `BehaviorModel.lean`. The `type` theorem proves against that rule, so it waits for CG-1.7.

## Proposed outcome
`docs/TASKS.md` sets TB-1.4 to `done` with this intent as its record, and adds sixteen `todo` rows under TB-1, TB-1.11 to TB-1.26. The plan gives each row's files, order, risk and proof. Each row names #16.

The rows follow the issue's recommendation, in this order of work:

- **Link 3 (C), proved relative to `BehaviorModel.lean`.** TB-1.11 defines a value semantics and proves the `length` theorem, the issue's first milestone. TB-1.15, TB-1.16, TB-1.17, TB-1.18 and TB-1.19 prove `nullability`, `range` with `rangeMin`, `choices`, `precision` and `type`. TB-1.13 adds the missing `max_digits` check first, so that TB-1.18 can state all three conjuncts of `decimalFieldAccepts`. TB-1.25 lifts the per-kind theorems to one end-to-end statement from exit 0 to acceptance by the model predicates.
- **Link 2 (A), proved relative to the IO shell and `leansqlite`.** TB-1.12 makes translation strict and pure: an unknown enum string, a dangling node or edge id, and an orphan contract row become exit 2, and `translateRows` returns `Except String ContractGraph`. TB-1.20 proves `translateRows`. TB-1.21 tests the IO shell by a round trip over every fixture database.
- **Link 1 and the residual of link 3 (D), tested.** TB-1.14 tests each `BehaviorModel.lean` predicate against pinned Django and pydantic. TB-1.22 tests the extractor against values observed in generated programs. TB-1.23 counts runtime writes to model fields that have no edge in the rows. TB-1.24 runs all three in CGV CI with recorded seeds and writes them into CGV's evidence record as `tested` claims, each with its count.
- **Link 1 (B), piloted.** TB-1.26 pilots certificate checking on one pattern, Django `CharField(max_length=N)`, and records what it cost.

The issue's four open questions are settled here as follows.

1. **D as the gate for link 1.** Rule 7 of `docs/VISION.md` decides this. D's result is a `tested` claim that states its count and seed. It is never shown as proof. A proof of link 1 is not yet reached. The property that blocks it is that the extractor is 16,715 lines of Rust with no semantics of the Python it reads. The open question is whether certificate checking (B) can cover the patterns at a cost that does not grow with each new pattern. TB-1.26 measures that cost.
2. **The subset for a B pilot.** One `model_extractor.rs` pattern, `CharField(max_length=N)`. It matches the `length` milestone and involves no value analysis. Whether the value analysis follows is decided from the pilot's measured cost, in the row that follows the pilot.
3. **Where the type rule goes.** Into `BehaviorModel.lean` first, through CG-1.7. TB-1.19 depends on CG-1.7.
4. **Completeness.** It gets its own row. TB-1.23 measures it as a `tested` claim. A proof of completeness is not yet reached. The property that blocks it is that a Python write can go through `setattr`, `__dict__`, `**kwargs`, `update(**d)` or a framework hook, so no syntactic walk enumerates the write sites. The open question is which Python subset, with which semantics, makes the set of writes enumerable.

This pull request changes no code and no protected surface.

## Affected users and systems
- Anyone who reads CGV's exit 0. Once the rows land, exit 0 means what the trust table states, with the residual trust in each link named.
- The agents and the maintainer who pick up TB-1 work through `node scripts/ci/task-queue.mjs next`.
- `docs/TASKS.md`, this intent, the plan, and the root `JOURNAL.md`. None is protected, so the change is Tier 1.

## Constraints
- Task IDs are never reused. TB-1.1 to TB-1.4 are on `origin/main`. Open pull requests #99 and #101 add TB-1.5 to TB-1.10. No other open pull request adds a TB row. So the new rows start at TB-1.11.
- A dependency must name a row already in the queue, or the task queue check fails. CG-1.17, which amends `pydanticDecimalAccepts`, is only in open pull request #90, so TB-1.18 names it in text and does not depend on it.
- Each new theorem that becomes part of the end-to-end guarantee goes into the table in `.claude/rules/protected-surfaces.md` and into `cgv/prover/scripts/ProtectedStatements.lean` in the same pull request. That makes each theorem row Tier 3, with a governance note and a **Protected-surface change** section.
- No LLM output counts as evidence for any row (rule 1). D's harnesses are deterministic given their seed (rule 3).
- What a row does not reach is named "not yet reached", with its blocking property and open question.

## Open questions
None for the split. Each row carries its own question for its own intent. TB-1.11 decides how a negative `Int` bound meets the `Nat` arguments of `charFieldAccepts`. TB-1.12 decides whether a `length` or `precision` row with no bound and no dependent expression is malformed or an absent constraint. TB-1.13 decides whether `max_digits` becomes a range requirement from the extractor (Tier 2) or a new check in `constraintImplies` (Tier 3). TB-1.22 decides whether the harness evaluates `satisfies` through a Lean executable or a Python copy. TB-1.25 decides the value semantics of a dependent-expression bound.
