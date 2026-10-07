# Plan: Prove CGV's extraction, link by link

Intent: `intent/2026-10-07-prove-extraction.md`
Governing roadmap item: TB-1. Task: TB-1.4. Issue: #16. Tier of this pull request: 1.

This plan is not the root `plan.md`, which belongs to an earlier change. Each row below opens its own intent, and its own spec where its tier needs one. The row's intent may change what this plan says about that row, and says so.

## This pull request

| File | Protected | Change |
|---|---|---|
| `intent/2026-10-07-prove-extraction.md` | no | new intent |
| `intent/2026-10-07-prove-extraction-plan.md` | no | this plan |
| `docs/TASKS.md` | no | TB-1.4 `done`; TB-1.11 to TB-1.26 added after it |
| `JOURNAL.md` | no | one entry |

Proof: the Task Queue Check and the pre-commit hook pass on the new rows, the Tier Gate passes at Tier 1, and no new ID appears on `origin/main` or in any open pull request's `docs/TASKS.md` diff.

## The three links and what each row leaves trusted

| Link | Rows | Strength once done | Residual trust |
|---|---|---|---|
| 3, graph to behaviour | TB-1.11, TB-1.13, TB-1.15 to TB-1.19, TB-1.25 | proved | `BehaviorModel.lean` |
| 3, the model itself | TB-1.14 | tested, with count and seed | the pinned Django and pydantic versions |
| 2, rows to graph | TB-1.12, TB-1.20 | proved | the IO shell (`readNodes`, `readContracts`, `readEdges`) and `leansqlite` |
| 2, the IO shell | TB-1.21 | tested over every fixture database | none beyond the fixtures |
| 1, source to rows | TB-1.22, TB-1.23, TB-1.24 | tested, with count and seed | the generator's grammar of patterns |
| 1, one pattern | TB-1.26 | checked by a second parse in Lean, not proved: a proof needs a soundness theorem for the checker, which the pilot does not require | the Python-subset semantics and the second parse |

## Order of work

The queue order is the order of work. Rows without a dependency between them can run in parallel.

1. **TB-1.11, value semantics and the `length` theorem.** Add a new module, `cgv/prover/ContractGraph/Semantics.lean`, imported by `ContractGraph.lean`. It defines `Value` (at least strings, `None`, scaled integers, decimals as digit counts) and `satisfies : Constraint → Value → Prop`. Prove: for a `length` guarantee `g` and a `length` requirement `r` with static bounds, `constraintImplies g r` and `satisfies g v` give `charFieldAccepts` and `pydanticMaxLengthAccepts` of `r`'s bound for `v`'s length. No `sorry`. Add the theorem to the table in `.claude/rules/protected-surfaces.md` and to `cgv/prover/scripts/ProtectedStatements.lean`, and regenerate `protected-statements.txt`. Tier 3. Done when `cd cgv/prover && lake build ContractGraph ContractGraph.Main` passes and the manifest check passes.
2. **TB-1.12, strict and pure translation.** In `Translation.lean`, make the four parsers return `Option`, and make `buildGraph` into `translateRows : List NodeRow → List ContractRow → List EdgeRow → Except String ContractGraph`. It throws on an unknown enum string, an edge whose endpoint id names no node, a contract row whose `node_id` names no node, and a contract row whose `edge_id` names no edge or an edge without `source_override`. `readContractGraph` stays the only IO. Add `#guard` cases in a new `ContractGraphTest/Translation.lean`, one per rejection and one accept case. Update the Translation row of the trust table in `cgv/README.md`. Tier 2, with the `#guard` cases as the spec.
3. **TB-1.13, check `max_digits`.** Make the fixture from the intent a test fixture with a `bad.py` (six integer digits into `max_digits=5, decimal_places=2`) and an `ok.py`. Then close the gap, either by the extractor emitting an upper and lower range requirement of `±(10^(m-d) - 10^-d)` next to the precision row, or by a new conjunct in `constraintImplies`. The first stays Tier 2 and reuses the existing range check, which is proved only against `constraintImplies` until TB-1.16 links it to `BehaviorModel.lean`. The second changes a protected definition and is Tier 3.
4. **TB-1.14, test the behaviour model.** Add `cgv/scripts/behavior_diff.py` and `cgv/scripts/behavior-requirements.txt` (pinned Django 4.2 and 5.x, pydantic 2, Hypothesis). For each predicate in `BehaviorModel.lean`, draw values with Hypothesis, run the real field's `clean` or pydantic validation, and compare the result with the predicate. The predicates are evaluated by a small Lean executable that reads JSON, so the comparison uses the Lean text and not a copy. Record the seed, the count per predicate, and every disagreement. A disagreement is a finding for a `BehaviorModel.lean` amendment, as CG-1.17 found by hand.
5. **TB-1.15 to TB-1.17, `nullability`, `range` with `rangeMin`, `choices`.** One theorem each, in `Semantics.lean`, against `notNullAccepts` and `annotationAcceptsNull`, `maxValueAcceptsMicros` and `minValueAcceptsMicros`, and `choicesAccepts`. The range theorem is relative to the scaled-integer claim in the docstring of `BehaviorModel.lean`, and says so. Each adds its theorem to the protected table and the manifest. Tier 3 each.
6. **TB-1.18, `precision`.** After TB-1.13, prove all three conjuncts of `decimalFieldAccepts` and of `pydanticDecimalAccepts`. If CG-1.17 has amended `pydanticDecimalAccepts` by then, prove against the amended rule. Tier 3.
7. **TB-1.19, `type`.** After CG-1.7, prove against the type rule CG-1.7 adds to `BehaviorModel.lean`. Tier 3.
8. **TB-1.20, prove `translateRows`.** State and prove: each node row gives exactly one node with the same id, name and kind, whose preconditions and postconditions are the translations of the contract rows with its id and role; each edge row gives exactly one edge; and `translateRows` returns `error` exactly when a row is malformed. Add the theorems to the protected table and the manifest. Tier 3.
9. **TB-1.21, test the IO shell.** Add a mode to the checker binary, or a separate Lean executable, that prints the rows it read as JSON. Compare that with the rows the Rust writer inserted, read back by `rusqlite`, over every database built from `cgv/test_fixtures/`. Tier 2.
10. **TB-1.22, test the extractor.** Add a generator of small Python programs from a grammar of the patterns the extractor handles, starting with Django `CharField(max_length=N)` writes and `Optional` returns. Run each program under Django with the test database, record each value that reaches a write site, and check that the value satisfies the guarantee the extractor wrote for that site. A value outside its extracted guarantee is an extractor bug. Use the same Lean evaluator as TB-1.14 for `satisfies`. Record the seed and the count.
11. **TB-1.23, count unseen writes.** In TB-1.22's runs, record every write to a model field at runtime, and count the writes whose site has no edge in the rows. The count is evidence about completeness, labelled `tested`.
12. **TB-1.24, CI and the evidence record.** Run TB-1.14, TB-1.21, TB-1.22 and TB-1.23 in CGV CI with fixed seeds and time limits, and add one `tested` claim per harness to the record that `contracts check --evidence-record` writes, with the count and the seed. `.github/workflows/**` is a Class A protected surface, so this row needs a governance note and Tier 3.
13. **TB-1.25, the end-to-end theorem.** Combine `runChecker_sound_all`, the per-kind theorems and the `translateRows` theorem: exit 0 on rows that translate means that, on every data path, every value the source guarantees admit is accepted by every `BehaviorModel.lean` predicate of the target. The open question for its intent is the value semantics of a dependent-expression bound. Update the trust table in `cgv/README.md`, the key theorems in `cgv/CLAUDE.md`, and the claim text of CGV's evidence record. Tier 3.
14. **TB-1.26, pilot certificate checking.** For `CharField(max_length=N)` only, have the extractor emit the class-body statement, its span and a rule id beside each fact. Add a Lean type for that fragment of the Python AST and a checker that re-derives the fact from it. Record the Lean lines, the extractor lines, the time per fixture, and what the Lean side still trusts (the second parse and the subset semantics). The pull request says whether the next pattern looks cheaper or not, with the numbers, and adds a row for the decision. Tier 2, or Tier 3 if it adds a protected theorem.

## Risks

- **Theorems that prove the wrong thing.** A `satisfies` that matches `constraintImplies` by construction proves nothing about behaviour. Each theorem must end in a `BehaviorModel.lean` predicate, never in another definition from `Checker.lean`. Review checks the conclusion of each statement.
- **The manifest churns.** Each theorem row regenerates `protected-statements.txt`. Rows that land in parallel conflict there. Merge `origin/main` and regenerate. Never hand-edit the file.
- **Strict translation breaks old databases.** TB-1.12 may reject a database that translates today. From `cgv/`, run `scripts/check-fixtures.sh` and `scripts/bench.py run --compare bench/baseline.json` before and after, and name every new exit 2 in the pull request.
- **Differential tests that cannot fail.** Each harness ships with a seeded mutant: a predicate, a guarantee or an edge changed on purpose, which the harness must report. Without it the harness is not merged.
- **Pinned framework versions drift.** The `tested` claims name the Django and pydantic versions. A version change is a new claim, not the same one.
- **Python in CGV CI.** TB-1.14 and TB-1.22 add Python packages to CGV CI. Pin them with hashes, and keep the run under the CI time budget, or run them on a schedule and say so in the claim.

## Proof that each row worked

- A theorem row: `lake build ContractGraph ContractGraph.Main` passes, the manifest check passes with the new statement in it, and the axiom check reports only `propext`, `Classical.choice` and `Quot.sound`.
- A translation row: the new `#guard` cases fail when the rejection is removed (checked by mutation), and every fixture keeps its verdict.
- A harness row: the harness passes on `main`, and fails on its seeded mutant with the seed it prints.
- TB-1.13: its `bad.py` gives an error on the `max_digits` write, and its `ok.py` gives none.
