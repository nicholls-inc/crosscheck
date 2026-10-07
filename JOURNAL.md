# JOURNAL.md

This is the repo-root journal — the broadest shard in the sharded-journal architecture described in [ADR-0001](docs/decisions/0001-sharded-journal-architecture.md). It records decisions that cut across the whole repo (plugins, marketplace conventions, tooling). Other shards live further down the tree at meaningful design boundaries and carry narrower decisions. Entries are newest first. Before non-trivial work in any directory, walk up reading every `JOURNAL.md` you pass — see [AGENTS.md](AGENTS.md) for the rule.

---

## 2026-10-07 - The CGV manifest hashes what the protected statements mention

**Type:** feat
**Touches:** cgv/prover/scripts/ProtectedStatements.lean, cgv/prover/protected-statements.txt, cgv/prover/scripts/manifest-selftest.sh, .github/workflows/cgv-ci.yml, .claude/rules/protected-surfaces.md, cgv/CLAUDE.md, cgv/README.md, intent/2026-09-29-deterministic-evidence-spec.md, docs/TASKS.md
**Why:** Issue #51. The manifest hashed only what `constraintImplies`, `IsDataPath` and `stepwiseSound` reach, so a redefinition of `CheckResult.isError` or `runChecker` could change what the theorems promise with no manifest change. The generator's name lists were kept in step with the rules table by hand.
**Links:** [intent](intent/2026-10-07-manifest-reach.md), [spec](intent/2026-10-07-manifest-reach-spec.md), [plan](intent/2026-10-07-manifest-reach-plan.md)

The walk now starts from every constant in a protected statement as well as the three definitions, and scopes by defining module, so private helpers count. The manifest grows from 107 hashed constants to 386, the checker among them, so a change to the checker's definitions now makes a pull request Tier 3. The generator reads the CGV table in `.claude/rules/protected-surfaces.md` and fails if names or files differ, and CGV CI runs when that file changes. `scripts/manifest-selftest.sh` pins each behaviour with a case that fails when it is mutated away. Whether that script is itself a protected surface is TB-1.27.

## 2026-10-07 - A project class named like a builtin type gets a type contract of its own

**Type:** fix
**Touches:** cgv/src/resolve.rs, cgv/src/function_extractor.rs, cgv/src/dataclass_extractor.rs, cgv/src/value_analysis.rs, cgv/src/extractor.rs, cgv/src/edge_discovery.rs, cgv/test_fixtures/shadowed_builtins/, cgv/README.md, docs/TASKS.md
**Why:** CGV read an annotation's type by the last segment of its name, so a project `class float` got the builtin's type contract. Writes between it and a builtin `float` passed with exit 0 (CG-1.15). The numeric tower in #87 would also let a project `int` into a `float` field.
**Links:** [intent](intent/2026-10-07-cgv-shadowed-builtins.md)

`ProjectIndex::annotation_shadow` resolves the type-contract names of an annotation (`int`, `float`, `str`, `bool`, `Decimal`, `Strict*`) in the module that holds it. A name bound to a project class gives a contract that names the class by its qualified name (`units.float`), which equals only itself, so neither equality nor the numeric tower matches it to a builtin. A name bound to anything else gives no type contract. The extractor applies the answer to parameter, return and data class field annotations, and `ProjectIndex::is_contract_type` replaces the `VALUE_TYPES` filters, so the qualified name survives into the database. No Lean file changes. One input now promises less: a name imported from a project module that re-exports the real type (`from compat import Decimal`) drops the type contract, so a wrong-typed write into it is no longer reported (CG-1.50). Names bound in a class body or a function, star imports from outside the project, aliases from another module and shadowed `con*()` helpers are not yet reached (CG-1.50).

## 2026-10-07 - A call through a subclass, and an enum's bases and decorators, are read as Python reads them

**Type:** fix
**Touches:** cgv/src/resolve.rs, cgv/src/exits.rs, cgv/src/extractor.rs, cgv/test_fixtures/no_return_resolution/, docs/TASKS.md
**Why:** pr-swarm found seven forms on CG-1.11 where a `NoReturn` method call or an enum `match` read as an exit although Python runs past it, so a real non-null error exited 0 (#5).
**Links:** [intent](intent/2026-10-07-method-resolution-exits.md)

`Sub.s(x)` is an exit only when `ProjectIndex::method_by_mro`, a C3 lookup that returns nothing when a base is unreadable or the binding class binds `s` other than by one `def`, names the same `NoReturn` method. `method` keeps its depth-first search, because edge discovery and value analysis use it and an uncertain result there would drop edges. An enum's bases must resolve through imports to `enum`, Django's choices classes or the builtins `str` and `int`. A method in an enum body may carry only a bare `staticmethod`, `classmethod` or `property` that the module does not rebind, which settles the row's open question with a fixed list. Each of the seven forms exited 0 before and exits 1 now. CG-1.48 takes the forms listed in its row. A nested class's bases are read in the enclosing class bodies, and a class body's `case` captures, walrus targets, `type` statements and statements in a `case` body count as bindings.

## 2026-10-07 - `max_digits` becomes a range requirement

**Type:** fix
**Touches:** cgv/src/bounds.rs, cgv/src/model_extractor.rs, cgv/src/dataclass_extractor.rs, cgv/scripts/max_digits_oracle.py, cgv/tests/e2e_text_format.rs, cgv/test_fixtures/tb_max_digits/, cgv/bench/baseline.json, cgv/README.md, docs/TASKS.md
**Why:** The checker compared only decimal places, so `Decimal("123456.78")` written into `DecimalField(max_digits=5, decimal_places=2)` exited 0 (measured on `main`, TB-1.13, #16).
**Links:** [intent](intent/2026-10-07-max-digits.md), [spec](intent/2026-10-07-max-digits-spec.md)

The extractor turns `max_digits=m` with `decimal_places=d` into a range requirement of `±(10^(m-d) - 10^-d)`, intersected with any validator bounds, for Django's `DecimalField` and for pydantic. With a readable `decimal_places`, together with the precision requirement this is `decimalFieldAccepts`, apart from the zero written into a field with `max_digits == decimal_places` and a `max_digits` above 38 or `decimal_places` above 30, which are not yet reached (see the spec's known gaps). A later `Field(...)` whose `decimal_places` is not a literal now makes it unknown, so a precision requirement inherited from an `Annotated` alias disappears for that field, and one whose `max_digits` is not a literal drops the inherited range. For a read that is fewer guarantees; for a write the checker accepts more values, so exit 0 promises less for such a field (TB-1.35 extends the rule to the other keywords). The task row offered a new conjunct in `constraintImplies` instead. It was not taken: no source has an integer-digit guarantee to compare against, while every literal write already has an exact range guarantee, so the range route needs no protected change and the range check already compares exact decimals. No fixture changes errors or exit code, and no bench outcome changes. Warnings rise on 37 fixtures, in pairs, one per side of the range for each write whose magnitude the extractor cannot bound. `scripts/max_digits_oracle.py` runs the extractor on generated Django and pydantic fields and checks each range against the real `DecimalValidator` and pydantic: 18,448 checks on Django 5.2.18 and pydantic 2.13.5, no failure. A read of such a field carries the range as a guarantee, as `max_length` does, so a sum of two reads written back and a copy into a narrower field are now errors; django-oscar 4.2.1 shows none, and TB-1.34 measures it on a larger codebase. The total digits of a field without a readable `decimal_places` are not yet reached (TB-1.33).

---

## 2026-10-07 - A context manager may suppress what a `with` body raises

**Type:** fix
**Touches:** cgv/src/flow.rs, cgv/test_fixtures/with_suppress/, docs/TASKS.md
**Why:** `with suppress(ValueError): raise ValueError(k)` at the end of a function, and `with suppress(AssertionError): assert x is not None` before `return x`, return None from a function annotated `-> str`, and CGV exited 0 on both.
**Links:** [intent](intent/2026-10-07-with-suppress.md)

`flow.rs` now assumes that any context manager may suppress any exception raised in its body, since it cannot tell which managers do. Inside a `with` body, at any depth, only a `return` ends the flow: a `raise`, a call that never returns and a `while True:` loop with no `break` do not. What a `with` body narrows never survives it, nor does what a later item of a multi-item `with` narrows (its expression runs inside the earlier managers); what the first item narrows does. The effects of the whole statement do survive. This replaces the CG-1.11 rule, which dropped the narrowing only when the body held a marked exit. The public bench corpus holds no `with`, so its result is unchanged. An exception raised before a `with` body's final `return` (or by the expression of a later item) under a suppressing manager is not yet reached, and so is keeping the narrowing of a manager that never suppresses. CG-1.47 takes both, through a closed list of managers that never suppress.

---

## 2026-10-07 - Baseline mode reports only the findings that a change introduces

**Type:** feature
**Touches:** cgv/src/baseline.rs, cgv/src/report.rs, cgv/src/main.rs, cgv/src/lib.rs, cgv/tests/e2e_baseline.rs, cgv/README.md, docs/TASKS.md
**Why:** A whole-project run on a real codebase reports about 240 errors, and a real bug ranked between #140 and #180. Nobody triages that on a pull request (#7).
**Links:** [intent](intent/2026-10-07-cgv-baseline-mode.md), [spec](intent/2026-10-07-cgv-baseline-mode-spec.md)

`--write-baseline PATH` writes a run's findings, and `--baseline PATH` prints only the findings that are not in the baseline, counts the rest, and lists the baseline findings that are gone. The key of a finding is its class, source, target, hop, bounds, site file and the normalised text of the site's line, with no line number, so an edit above a finding does not make it new. The comparison counts each key, and when a key's count rises every finding with that key is printed as new. Baseline mode exits 1 when any error is unmatched, so it can only hide a finding that has an equal key in the baseline. A finding swapped for an equal one is not yet reached, and the spec names the open question. Exit 0 in baseline mode does not carry `runChecker_sound_all`, so `--baseline` refuses `--evidence-record`. It also refuses a `--write-baseline` that names the same file, which would overwrite the baseline with the run's new errors. Without either flag the output is unchanged. On the 10 cases of the public bench corpus, a baseline from `fix/` compared with `pre/` exits 1 on exactly the 5 cases whose `pre/` has an error that `fix/` lacks.

## 2026-10-07 - Proving CGV's extraction is split into sixteen rows

**Type:** docs
**Touches:** intent/2026-10-07-prove-extraction.md, intent/2026-10-07-prove-extraction-plan.md, docs/TASKS.md
**Why:** #16 names three unproved links between Python source and the graph that `runChecker_sound_all` is about, and four approaches to them, but the queue had one row for all of it.
**Links:** [intent](intent/2026-10-07-prove-extraction.md), [plan](intent/2026-10-07-prove-extraction-plan.md)

TB-1.11 to TB-1.26 follow the issue's recommendation. Link 3 is proved per constraint kind against `BehaviorModel.lean`, then end to end. Link 2 is made strict and pure, then proved, and its IO shell is tested. Link 1 is tested against values observed in generated programs, and certificate checking is piloted on one pattern to measure its cost. Reading the code found that `param_max_digits` is written by the extractor and dropped by `Translation.lean`, so a write with too many integer digits into a `DecimalField` exits 0 (measured on a two-file fixture with binaries built from earlier task branches, not yet re-measured on `main`), and that `buildGraph` drops an edge whose endpoint names no node, which the README's trust table contradicts by saying malformed rows exit 2. TB-1.12 corrects that row. TB-1.13 and TB-1.12 cover them. A proof of link 1 and a proof of completeness stay not yet reached, and the intent names the blocking property and the open question for each.

---

## 2026-10-07 - The text report counts a missing guarantee as coverage instead of printing it

**Type:** feature
**Touches:** cgv/src/report.rs, cgv/src/main.rs, cgv/tests/e2e_text_format.rs, cgv/tests/e2e_cli.rs, cgv/README.md, docs/TASKS.md
**Why:** On a private Django codebase, 7,595 of 8,374 warnings said only that a source had no nullability guarantee. Readers took them for findings, and the errors were lost among them (#9).
**Links:** [intent](intent/2026-10-07-cgv-unverified-coverage.md), [spec](intent/2026-10-07-cgv-unverified-coverage-spec.md)

`--format text` sorts each checker result into one of four classes: incomplete, error, unverified and warning. A warning whose `source_guarantee` reads `<kind> (unspecified)` is unverified. By default the report prints no block for it, and counts it in a new `COVERAGE BY MODULE` section and in the `RESULT:` line, so a run that exits 0 with unverified requirements still says so. The section names the blocking property and the open question. `--warnings` prints the blocks, labelled `UNVERIFIED`. The edge counts come from the contract database the checker read, and on every fixture they sum to `edges_checked`. JSON and the exit code are unchanged. The class rests on the checker's wording, and a warning the report cannot classify is printed, not hidden. CG-1.31 moves the class into the checker's JSON and counts checked hop states for each module.

---

## 2026-10-07 - The imported Crosscheck backlog has one decision per issue

**Type:** docs
**Touches:** intent/2026-10-07-backlog-review.md, intent/2026-10-07-backlog-review-decisions.md, docs/TASKS.md
**Why:** AD-1 asks for one decision, Refine or Drop, on each of the 23 issues (#19 to #41) that came with Crosscheck. They were written before the vision, and none had a row in the queue.
**Links:** [intent](intent/2026-10-07-backlog-review.md), [decisions](intent/2026-10-07-backlog-review-decisions.md)

Twenty-one issues are refined and two are dropped. #37 is already done on `main`, and #34 is done apart from two residues that move to AD-1.11, and the decisions file cites the lines that do them. The refined issues give 19 new rows: PB-1.18 to PB-1.21 for Crosscheck's own deterministic CI checks, VA-1.9 to VA-1.11 for agents and skills that present an unchecked or LLM verdict as evidence, and AD-1.2 to AD-1.13 for work that serves a design rule and no other current item. Two refinements change what an issue asked for, because the original conflicts with rule 1 of the vision. #22 no longer makes an LLM judge the pass condition of the acceptance oracles. #32 no longer lets an LLM's tag close a finding. The GitHub issues are rewritten and closed from the decisions file after review, not in this pull request.

---

## 2026-10-07 - CGV CI replays the built environment through the Lean kernel

**Type:** feature
**Touches:** .github/workflows/cgv-ci.yml, .claude/rules/protected-surfaces.md, docs/assurance/DEVELOPMENT-FRAMEWORK.md, cgv/CLAUDE.md, cgv/README.md, intent/2026-09-29-deterministic-evidence-spec.md, docs/TASKS.md
**Why:** The generator's axiom check uses `collectAxioms`, which does not re-check a proof. A theorem of `False` added under `debug.skipKernelTC` with `addDecl` kept `lake build` green, left the manifest unchanged and passed the axiom check (#47).
**Links:** [intent](intent/2026-10-07-kernel-replay.md), [plan](intent/2026-10-07-kernel-replay-plan.md)

The pinned toolchain ships `leanchecker`, so no dependency was added. `leanchecker --fresh ContractGraph.Main` replays the whole import closure of `ContractGraph.Main` (the Lean library, `leansqlite` and the `ContractGraph` modules) into an empty environment, and `leanchecker ContractGraph` replays every `ContractGraph.*` module. Locally the first took 48 to 57 s and the second 3.5 s. A self-test step compiles a bad module and a module that imports it outside the Lake package, and fails unless both modes reject them with the kernel's type mismatch, so a toolchain whose `leanchecker` changes cannot pass silently. The self-test sets `PATH` to the pinned toolchain's `bin`: run from a scratch directory, `leanchecker` otherwise asked elan's default toolchain for its sysroot and failed on an incompatible `Init.olean`. The replay skips constants whose kernel safety is `unsafe` or `partial`, and a safe theorem that uses one fails with an unknown constant. It uses the kernel that built the files, so a second, independent checker stays not yet reached: it needs an export of the proofs that a second checker reads, and the open question is which checker to use. The tier gate's evidence line for `cgv/**` does not name the replay yet (TB-1.5).

---

## 2026-10-06 - A call that never returns, and a match over every enum member, end the flow

**Type:** fix
**Touches:** cgv/README.md, cgv/src/exits.rs, cgv/src/flow.rs, cgv/src/resolve.rs, cgv/src/extractor.rs, cgv/src/function_extractor.rs, cgv/src/edge_discovery.rs, cgv/src/value_analysis.rs, cgv/test_fixtures/no_return_exits/, cgv/bench/baseline.json, docs/TASKS.md
**Why:** CGV treated a body that ends in `assert_never(x)`, `sys.exit(1)` or a `match` over every member of an enum as falling through to `return None`, a false non-null error against a non-Optional return. It was 2 of the 20 triaged non-null errors in the real-codebase evaluation, and the `no-return` false positive in the labelled benchmark.
**Links:** [intent](intent/2026-10-06-no-return-exits.md)

`flow.rs` cannot resolve a name, so the decision is made once per function after the project index is complete. `exits::function_exits` returns an `Exits`, two sets of statement offsets kept on `FunctionInfo`, and `always_exits`, `terminates` and `walk` read it. A call counts when it reaches `sys.exit`, `os._exit`, `os.abort`, `typing.assert_never` or `typing_extensions.assert_never` through a non-project import, the builtins `exit` or `quit`, or a project function annotated `NoReturn` or `Never` with no wrapping decorator, awaited exactly when it is `async`. A match counts when its subject is an unrebound parameter annotated with a project enum whose member list is certain, and unguarded `E.NAME` patterns name every member. A name the function binds, a module global the module rebinds, and a name reached through an import chain in which a module rebinds it, or imports it from two places, and a name that a module with a star import neither imports nor defines, never count (a star import after an explicit import is CG-1.22). Anything unrecognised is assumed to complete, which can only add errors. Instance method calls, attribute or local subjects, and enums or `NoReturn` functions from outside the project are not yet reached, and the intent names the blocking property and the open question for each.

---

## 2026-10-06 - CGV narrows None in four more patterns

**Type:** fix
**Touches:** cgv/src/flow.rs, cgv/src/value_analysis.rs, cgv/src/edge_discovery.rs, cgv/test_fixtures/cg_narrowing/, cgv/test_fixtures/cg_narrowing_getattr/, cgv/test_fixtures/cg_narrowing_globals/, cgv/bench/baseline.json, cgv/README.md, intent/2026-10-06-narrowing-patterns.md, docs/TASKS.md
**Why:** 4 of the 20 triaged non-null errors on the evaluated codebase came from patterns the walk did not narrow (issue #5, CG-1.9).
**Links:** [intent](intent/2026-10-06-narrowing-patterns.md), [fixture](cgv/test_fixtures/cg_narrowing/expected.json)

After a `try`, the walk now joins the paths that fall through, so `try: v = int(v)` with a handler that returns keeps `v` non-None. `x in {"a", "b"}` narrows `x` when every element is a non-None literal. `k in d` is a fact kept among the narrowed names as `d[k]`. Under it, `d.get(k)` has the facts of `d[k]`, so a warning rather than an error. The fact is dropped when `d` or `k` is rebound, after any call other than `.get`, a `del`, a walrus or a suspension (`await`, `yield`, `async for`, `async with`, an async comprehension), and on entry to a loop, a `try` handler, a `finally`, a `match` statement or a definition (decorator, default or class body) that has one. A read of `p.f` through a parameter has unknown nullability when every call of the function narrows `arg.f` and the project shows every caller: at least one call from outside the function and outside any cycle of calls that nothing else enters. A pre-pass in `edge_discovery::caller_guards` decides that, and it excludes an async function and a generator, whose body runs after the call, when the caller may have written the field. A `while` test is read after the loop's own effects, since it runs again after every iteration.

The maintainer then settled the review's open questions with one rule: exit 0 promises consistency, so a narrowing that can hide a real nullability bug is a soundness hole, and soundness wins over precision where it is affordable. So a computed `getattr` switches caller guards off everywhere, and `globals()` or `locals()` in the module that calls it. A guard ends at the first call that can reach the parameter. `d.get(k)` narrows only when `d` is known to be a dict, only `d`'s own `.get` keeps the fact, and the fact also dies between operands of `and` / `or`, across a conditional expression's test and across a comprehension. The new near misses in `cg_narrowing/bugs.py` and the fixtures `cg_narrowing_getattr/` and `cg_narrowing_globals/` each fail when their fix is reverted. The pattern 2 and pattern 4 rules lower an error to a warning, never to silence, because a caller outside the project or a stored `None` can still reach the read. On the bench, `labelled-app` precision rose from 0.2857 to 0.3333. The fixed false positive now shows as a stale label, and the baseline is refreshed.

---

## 2026-10-06 - pydantic `model_validate` is a validation boundary

**Type:** fix
**Touches:** cgv/src/edge_discovery.rs, cgv/src/dataclass_extractor.rs, cgv/src/resolve.rs, cgv/src/extractor.rs, cgv/test_fixtures/pydantic_validate_boundary/, cgv/README.md, cgv/docs/design/dataflow-v2.md, docs/TASKS.md
**Why:** CGV checked each entry of the dict given to `Model.model_validate` against the field, as if it were a typed constructor argument. `model_validate` takes `Any`, coerces in lax mode and rejects invalid input on purpose, so the errors were false. It was 4 of the triaged false positives in the real-codebase evaluation.
**Links:** [intent](intent/2026-10-06-pydantic-validate-boundary.md)

At `model_validate`, `model_validate_json`, `model_validate_strings`, `parse_obj` and `parse_raw` on a pydantic class, an entry is no longer a write when validation enforces the field's contract. The first draft dropped every entry. Review and a pydantic 2.13.5 run showed that this left reads of some fields resting on a contract nothing checked: `decimal_places` and `max_digits` count digits after trailing zeros are dropped, and `SkipValidation`, `PlainValidator` and `WrapValidator` can store None in a `str` field. Those fields, and a `None` default that pydantic v1 reads as Optional, keep their writes. A later review found that matching those markers by literal name lets a renamed import or an alias drop the write, so the rule is now an allowlist: a field counts as validated only when every name in its annotation, every union member and every `Annotated` metadata item is on a list of types and constraints that pydantic validates, and anything else keeps the write. A project name that shadows a listed name is still read as the listed type (CG-1.43), and container arguments and aliased validator decorators are not inspected (CG-1.44). So `r5_write_patterns` keeps its `model_validate` precision error. A typed constructor call and `model_copy(update=...)` stay writes. `BehaviorModel.lean` needs no rule for lax coercion, but its `pydanticDecimalAccepts` is false for trailing zeros (CG-1.17), and `Annotated` validators do not clear requirements as the decorator forms do (CG-1.18). `model_construct` records no write (CG-1.16).

---

## 2026-10-06 - A parameter typed `object` accepts None

**Type:** fix
**Touches:** cgv/src/function_extractor.rs, cgv/test_fixtures/object_param_none/, docs/TASKS.md
**Why:** CGV reported an error when a caller passed an Optional value to a parameter typed `object`. `None` is an instance of `object`, so the error was false. It was 4 of the 20 triaged non-null errors in the real-codebase evaluation, and one of the four false positives in the labelled benchmark.
**Links:** [intent](intent/2026-10-06-object-param-accepts-none.md)

A parameter whose annotation names `object` now has the nullability of `Optional[object]`. So does a parameter typed with a union that has an `object` member, such as `Union[object, int]` or `object | int`, since that union is `object`. That check lives on the parameter path, and the shared union walk is unchanged. It gets no non-null precondition, and inside the function its value may be None, so a write of it into a non-null field is an error. `Any` and an unannotated parameter already gave no precondition, and the new fixture pins all three. Data class fields typed `object` stay non-null. Several fixtures use that as a non-null field with no type contract, and the docstring of `annotationAcceptsNull` in `BehaviorModel.lean` states the same rule, so the change to fields is CG-1.12.

---

## 2026-10-06 - A checker reads an evidence record and says which rules it breaks

**Type:** feature
**Touches:** scripts/check-evidence-record.mjs, scripts/check-evidence-record.test.mjs, intent/2026-10-06-evidence-record-checker.md, docs/TASKS.md
**Why:** ER-1's acceptance asks for a deterministic checker that rejects a claim with no strength or no rerun command. The spec's rules EV-1 to EV-13 had no implementation.
**Links:** [intent](intent/2026-10-06-evidence-record-checker.md), [spec](intent/2026-10-06-evidence-record-spec.md)

`node scripts/check-evidence-record.mjs <path>` exits 0, 1 with one `EV-N:` line per broken rule, or 2 when the file is missing or not JSON. The rules live in the pure `checkRecord`, and one table maps each strength to its basis fields, so EV-9 and EV-10 read the same source. Three runtime defaults disagree with the spec, so the checker avoids them: `String.prototype.trim` strips U+FEFF and keeps U+0085, `TextDecoder` drops a leading byte order mark unless `ignoreBOM` is set, and `Number.isInteger` accepts 1e30. By hand, not by a committed test, a record that CGV's `--evidence-record` wrote (ER-1.2, #79) passed, and the same record with `strength` and `rerun.command` deleted failed on EV-5, EV-9 and EV-12. ER-1.2 has merged, so that record can now be committed as a fixture. The checker checks shape, not truth: it runs no command from the record. No workflow runs its tests yet, because the tier gate runs only `scripts/ci/*.test.mjs`. ER-1.5 adds a CI step.

---

## 2026-10-06 - CGV writes an evidence record

**Type:** feature
**Touches:** cgv/src/evidence.rs, cgv/src/main.rs, cgv/src/report.rs, cgv/build.rs, cgv/Cargo.toml, cgv/tests/e2e_evidence.rs, cgv/README.md, docs/TASKS.md
**Why:** Exit 0 rests on `runChecker_sound_all`, but CGV's output named no theorem, commit, trusted base or rerun command. ER-1 asks CGV to emit a record in the format of ER-1.1.
**Links:** [intent](intent/2026-10-06-cgv-evidence-record.md), [spec](intent/2026-10-06-cgv-evidence-record-spec.md)

`contracts check --evidence-record PATH` writes one `proved` claim after a run that exits 0, and removes PATH after any other outcome. The commit comes from git, and CGV refuses (exit 2) a checked path or overrides file with uncommitted or untracked changes. The trusted base keeps `BehaviorModel.lean`, adds the three permitted axioms, pins the checker binary by SHA-256, and lists the docstring contracts tagged `ASSUMED` with their count. The rerun command repeats the run's options from the work-tree root, and names the checker by a local absolute path, so another machine must edit it. The record pins `Translation.lean` to the CLI's build commit, which matches the checker's sources only when both come from one checkout.

---

## 2026-10-06 - The evidence record has a format

**Type:** docs
**Touches:** intent/2026-10-06-evidence-record.md, intent/2026-10-06-evidence-record-spec.md, docs/TASKS.md
**Why:** The vision says every change ships with a record of evidence, and ER-1.2 to ER-1.4 need one format to emit and check. Neither tool said how strong its result was, what it trusted, or how to rerun it.
**Links:** [intent](intent/2026-10-06-evidence-record.md), [spec](intent/2026-10-06-evidence-record-spec.md)

A record is a closed JSON object about one commit. Each claim names its strength (`proved`, `tested`, `observed` or `judged`), a basis whose fields depend on the strength, a trusted base of pinned components, a rerun command with its exit code, and a requirement or an explicit `null`. Rules EV-1 to EV-12 are decidable from the record alone, so the ER-1.4 checker needs no network, no LLM and runs no command. The record has no overall verdict, because how strengths combine is an open question of the vision. The checker checks shape, not truth. Rerunning every claim, proving that a judge is a person, recording a proof's axioms and a second independent checker, recording that a person approved a requirement, and pinning the Crosscheck Docker images by digest are not yet reached. The spec's "Concerns flagged" section names the blocking property and the open question for each, together with the null seed, the vacuous rerun for `observed` and `judged` claims, the record that cannot sit in its own commit, and CGV's contract levels.

---

## 2026-10-06 - A pre-commit hook runs the checks that need no PR body

**Type:** feature
**Touches:** .husky/pre-commit, scripts/ci/pre-commit.mjs, scripts/ci/pre-commit.test.mjs, scripts/ci/tier-gate.mjs, scripts/ci/tier-gate.test.mjs, scripts/ci/task-queue.mjs, docs/assurance/DEVELOPMENT-FRAMEWORK.md, docs/assurance/TIER-LAYER-MAP.md, docs/gates/tier-layer-gate.md, docs/gates/task-queue-check.md, docs/TASKS.md
**Why:** The roadmap's dual-track principle asks every deterministic check for a pre-commit hook as well as a CI job. The tier gate and the task queue check had only the CI job, and the PreToolUse hook sees only Claude Code's edit tools.
**Links:** [intent](intent/2026-10-06-pre-commit-hooks.md), [spec](intent/2026-10-06-pre-commit-hooks-spec.md), [plan](intent/2026-10-06-pre-commit-hooks-plan.md)

`.husky/pre-commit` runs `node scripts/ci/pre-commit.mjs` on the commit as staged. A commit that stages a protected path fails unless every protected path the branch changes is named in a governance note the branch changes, which is the tier gate's TG-5 with the branch set taken from the index against the merge base. A commit that stages the queue or the roadmap fails on QC-1 to QC-4. The `Task:` line, the tier declaration, the citations and the CGV section live in the PR body, so they stay in CI. The hook reuses the CI scripts' own functions, so the two enforcement points cannot drift apart on the rules they share. It never fetches, so a stale `origin/main` can make it disagree with CI, and `--no-verify` skips it. On this repository a failing commit took 0.26 s.

---

## 2026-10-06 - The Tier Gate reads changed file names NUL-separated

**Type:** fix
**Touches:** .github/workflows/tier-gate.yml, scripts/ci/tier-gate.mjs, scripts/ci/tier-gate-workflow.test.mjs, intent/2026-09-29-deterministic-evidence-spec.md, docs/TASKS.md
**Why:** `git diff --name-only` C-quotes a name with a non-ASCII byte, a double quote, a backslash or a control character. The quoted name matched no protected glob, so `docs/assurance/é.md` passed the gate at `Tier: 1`.
**Links:** [intent](intent/2026-10-06-unquoted-paths.md), [spec](intent/2026-10-06-unquoted-paths-spec.md), [plan](intent/2026-10-06-unquoted-paths-plan.md)

The step now writes `git diff -z` to a temporary file and passes its path as `CHANGED_FILES_PATH`. The gate splits on NUL and keeps each name as written. `core.quotePath=false` was not enough, since it still quotes `"`, `\` and control characters. Once a name with a newline reached the gate intact, `**` compiled to `.*` still stopped at the newline, so each glob now compiles with the `s` flag. `.husky/commit-msg` has the quoting fault (PB-1.13), and the protected-surface hook has the newline fault (PB-1.14). The gate still reads an unset `CHANGED_FILES_PATH` as an empty list, which fails open. The maintainer kept that out of this task, and PB-1.15 makes it fail closed.

---

## 2026-10-06 - The tier gate's explainer stops claiming it checks incident evals

**Type:** docs
**Touches:** docs/gates/tier-layer-gate.md, docs/TASKS.md
**Why:** The explainer said the tier gate expects an incident's eval before it passes. The tier gate reads no incident reference. A run with the `incident` label, an incident id and no eval passes it.
**Links:** [intent](intent/2026-10-06-tier-gate-incident-doc.md)

The section now says that the Incident Eval Check, a separate workflow, checks incidents, names its trigger and the eval and candidate invariant it needs, and says that it runs after the merge and cannot block it. The run for #62 failed after its merge because the pull request quoted the trigger in prose. Task PB-1.16 covers that.

---

## 2026-10-06 - The framework states what the Incident Eval Check does

**Type:** docs
**Touches:** docs/assurance/DEVELOPMENT-FRAMEWORK.md, docs/TASKS.md
**Why:** Stage 5 said the check fails on an incident record under `evals/` with no eval. The check never looks for incident records, needs a candidate invariant too, has an exit 2, and runs only after the merge.
**Links:** [intent](intent/2026-10-06-incident-eval-doc.md), [plan](intent/2026-10-06-incident-eval-doc-plan.md)

The bullet now names the trigger (the `incident` label, or a `Fixes-Incident:` line in the body or a commit), the eval and the candidate invariant it needs, exit 1, and exit 2 for commits it cannot read. It also says that the workflow runs on a merged pull request, so it reports on a merge and cannot block one. The run for #61 started four seconds after the merge, and the run for #60, closed without a merge, was skipped. `docs/gates/tier-layer-gate.md` still says the tier gate expects the eval before it passes. Task PB-1.11 fixes that.

---

## 2026-10-01 - The Tier Gate step computes its own changed files

**Type:** fix
**Touches:** .github/workflows/tier-gate.yml, scripts/ci/tier-gate-workflow.test.mjs, intent/2026-09-29-deterministic-evidence-spec.md, docs/TASKS.md
**Why:** A changed file named `EOF` ended the workflow's `$GITHUB_OUTPUT` block early. With a second file named `docs/assurance/a<<EOF`, scratch pull request #60 changed a protected file at `Tier: 1` and the gate passed on an empty list.
**Links:** [intent](intent/2026-10-01-tier-gate-workflow.md), [spec](intent/2026-10-01-tier-gate-workflow-spec.md), [plan](intent/2026-10-01-tier-gate-workflow-plan.md)

The gate step now fetches the base, sets `CHANGED_FILES` from `git diff` and runs the gate, so no file name passes through `$GITHUB_OUTPUT`. The base ref reaches the script as `BASE_REF` in `env:`, not as a `${{ }}` expression in shell source, and the gate's pass line now names it. A new test runs the step's script from the workflow file in a scratch repository, which also tests `--no-renames` for the first time. The test runs under local bash, not on a GitHub runner, so the runner evidence is #60 and this change's own Tier Gate run. `git diff --name-only` still quotes a path with a non-ASCII byte, which then matches no protected glob. Task PB-1.10 fixes that.

---

## 2026-10-01 - A script picks the next task, and CI checks the queue

**Type:** feature
**Touches:** scripts/ci/task-queue.mjs, scripts/ci/task-queue.test.mjs, .github/workflows/task-queue.yml, docs/assurance/DEVELOPMENT-FRAMEWORK.md, docs/gates/task-queue-check.md, docs/gates/README.md, docs/TASKS.md
**Why:** Agents read the queue by eye, nothing checked it, and a pull request could mark any row `done`. The queue's own spec named PB-1.6 as the fix.
**Links:** [intent](intent/2026-10-01-queue-check.md), [spec](intent/2026-10-01-queue-check-spec.md), [plan](intent/2026-10-01-queue-check-plan.md)

`node scripts/ci/task-queue.mjs next` applies the three conditions of the pick-up procedure, so two agents can no longer read the table two ways. `check` runs on every pull request. It fails on a task ID with no roadmap item, a repeated ID, an unknown status, or a missing dependency, and when a pull request sets to `done` a row its `Task:` line does not name. The `Task:` line uses the tier gate's anchor, so quoted text cannot name a task. The claim snippet stays in the framework document, and the race test runs that snippet itself rather than a copy, so an edit that breaks the claim breaks the test. The check does not require the completing pull request to set its own row, because the split rule lets a pull request replace a row and set nothing to `done`.

---

## 2026-09-30 - Any citation line counts, and only a regular file in the repository

**Type:** fix
**Touches:** scripts/ci/tier-gate.mjs, scripts/ci/tier-gate.test.mjs, docs/assurance/TIER-LAYER-MAP.md, docs/gates/tier-layer-gate.md, intent/2026-09-29-deterministic-evidence-spec.md, docs/TASKS.md
**Why:** Only the first `Plan:` line counted, and `existsSync` accepted a directory, `../x`, or a symlink out of the repository (#49).
**Links:** [intent](intent/2026-09-30-citation-rule.md), [spec](intent/2026-09-30-citation-rule-spec.md), [plan](intent/2026-09-30-citation-rule-plan.md)

The gate now reads every `Intent:`, `Spec:` or `Plan:` line, and one valid line meets the requirement. A cited path must resolve, through symlinks, to a regular file whose real path is inside the repository. The line format is unchanged. The gate still checks only that the file exists, not what it says, so any file in the repository satisfies a citation. That gap is recorded in TG-12 and is not new.

---

## 2026-09-30 - The tier gate reads `Tier:` only at the start of a line

**Type:** fix
**Touches:** scripts/ci/tier-gate.mjs, scripts/ci/tier-gate.test.mjs, docs/assurance/TIER-LAYER-MAP.md, docs/gates/tier-layer-gate.md, intent/2026-09-29-deterministic-evidence-spec.md, docs/TASKS.md
**Why:** Pasted text such as "Tier: 1" could declare a tier, and the pass report called unchecked code "none required at this tier" (#50). The maintainer's review decisions on #57 set the stricter rules.
**Links:** [intent](intent/2026-09-30-tier-anchor.md), [spec](intent/2026-09-30-tier-anchor-spec.md), [plan](intent/2026-09-30-tier-anchor-plan.md)

A `Tier:` line now starts the line, after an optional indent and no list or quote marker. The first `Tier:` line is the declaration, and an invalid one fails. A `Tier:` line and a `tier:N` label must agree: the map always said so, and the gate now enforces it. The pass report's classes are now rows of two kinds. A checked row names its workflow. A not-yet-reached row names the blocking property and the open question. Prose, `CLAUDE.md`, `AGENTS.md`, `REVIEW.md` and root `docs/invariants/**` are not yet reached too, so no line says "none required at this tier". The citations keep their list and quote markers, because a citation only names an existing file and cannot pick the tier. Two paths were misreported and now name their checks: `crosscheck/conformance/**` (the conformance job) and the protected-surface hook (its tests in the Tier Gate job).

---

## 2026-09-30 — Merged governance notes no longer unlock the hook

**Type:** fix
**Touches:** .claude/hooks/protected-surface-guard.mjs, scripts/ci/protected-surface-guard.test.mjs, .claude/rules/protected-surfaces.md, docs/assurance/DEVELOPMENT-FRAMEWORK.md, docs/gates/protected-surface-hook.md, crosscheck/skills/assurance-init/SKILL.md, docs/TASKS.md, CLAUDE.md
**Why:** On main at 608ca86 the hook allowed edits to 30 of 58 protected files with no new note because it counted notes that had already merged to the default branch.
**Links:** [intent](intent/2026-09-30-merged-notes-unlock.md), [spec](intent/2026-09-30-merged-notes-unlock-spec.md), [plan](intent/2026-09-30-merged-notes-unlock-plan.md)

The hook now compares a governance note's text on this branch against the default branch (origin/HEAD else origin/main). Only notes that are new on this branch count; notes already merged to main no longer unlock edits. This mirrors the Tier Gate's rule that a note from an earlier change does not count.

---

## 2026-09-30 — A task queue beside the roadmap

**Type:** intent-refinement
**Touches:** docs/TASKS.md (new), docs/assurance/ROADMAP.md, docs/assurance/DEVELOPMENT-FRAMEWORK.md, CLAUDE.md, AGENTS.md
**Why:** Each new session had to be told where the work stood, and the roadmap had no item for any part of the vision.
**Links:** [intent](intent/2026-09-30-task-queue.md), [spec](intent/2026-09-30-task-queue-spec.md), [queue](docs/TASKS.md)

The roadmap now has an item for each part of the vision, and `docs/TASKS.md` is the ordered queue of tasks under those items. The two are separate files on purpose. The roadmap is a protected surface, so a status change there is a Tier 3 change with a governance note. The queue is not protected, so the pull request that finishes a task can mark it done at whatever tier the task has. The queue has no "in progress" status, because a status on a branch is invisible to every other branch. A pushed branch named `task/<task ID>` is the claim. It holds one empty commit with a unique message and is pushed with a lease that fails if the branch exists, because a plain push of a branch cut from `origin/main` succeeds a second time with "Everything up-to-date". Nothing deterministic checks the queue yet, and task PB-1.6 adds that check.

## 2026-05-11 — Sharded journals plus a root walk-up rule [ADR-0001]

**Type:** intent-refinement
**Touches:** AGENTS.md (new), JOURNAL.md shards (new at repo root, crosscheck/, crosscheck/docs/add/), docs/decisions/ (new)
**Why:** We wanted a shared narrative record that humans and agents both read by default, and the first try inside the Crosscheck plugin was archived after one shipped iteration.
**Links:** [ADR-0001](docs/decisions/0001-sharded-journal-architecture.md), [v2 retrospective](crosscheck/docs/add/.retrospective/findings-and-methodology-v2.md)

This repo is starting to use co-located `JOURNAL.md` files plus a root `AGENTS.md` walk-up rule. The first place it lands is the Crosscheck plugin's design work, which is where the need surfaced. The shape is small on purpose — a header per file, one entry per decision, plain product voice, frontmatter for type and links. It may change once it gets driven against real spec sessions; the retrospective is candid that nothing about the working hypothesis is settled yet.
