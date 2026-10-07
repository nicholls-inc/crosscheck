# Intent: CGV narrows nullability in four more patterns

Task: CG-1.9. Governing roadmap item: CG-1. Issue: #5.

## Problem statement
On the codebase that `cgv/docs/evaluation/real-codebase-evaluation-2026-09.md` measured, 4 of the 20 triaged non-null errors were false, and each came from a pattern that `cgv/src/flow.rs` does not narrow. Each pattern reproduces on `main` in `cgv/test_fixtures/cg_narrowing/ok.py`, where all ten safe functions give a non-null error.

1. **Reassignment after an early return, inside `try`.** `if not v: return` then `try: v = int(v)` / `except ValueError: return`. After a `try`, the walk forgets every name the `try` binds, even when the handler exits. A read of `v` then joins every assignment in the function, `v = None` included.
2. **A caller's guard on an attribute.** `if h.name: label_of(h)`, where `label_of` writes `h.name` into a `str` field. A field read gives the field's declared contract (`Optional[str]`), whatever the callers check.
3. **`x in {literals}`.** `if v in {"a", "b"}:` does not narrow `v`, though no non-None literal equals None.
4. **`k in d` before `d.get(k)`.** `.get(k)` with one argument is always nullable, even where `k in d` holds.

## Proposed outcome
`ok.py` passes, and every near miss in `cgv/test_fixtures/cg_narrowing/bugs.py` stays an error. `expected.json` lists the errors of `bugs.py` and nothing from `ok.py`, and requires the four warnings that `ok.py` yields for patterns 2 and 4.

1. **`try`.** After a `try` statement, the state is the join of the paths that fall through: the body followed by `else`, and each handler that does not exit. A handler starts from the state before the `try` minus every name the `try` binds, as today. When the `try` has a `finally`, the `finally` body starts from the join of that state and the fall-through join, since it also runs on exceptional paths, and its end is the state after the statement.
2. **Caller guards.** A read of `p.f`, where `p` is a parameter that the function has not rebound and `f`'s declared contract is nullable, has unknown nullability when every call of the function in the project narrows `arg.f` at the call. Unknown gives a warning where a non-null target needs it, never an error. The rule applies only to functions the project can see every caller of: at least one call from outside the function and outside any cycle of calls that nothing else enters, every call by the function's name resolves to it, no use of the function's name other than as a call, no decorator, not a dunder method, and for a method, a class whose bases are all project classes. A call that passes `*args` or `**kwargs` counts as unguarded.
3. **Literals.** `x in <set, list or tuple display>` narrows `x` when every element is a string, bytes, number or bool literal and the display is not empty. `x not in` narrows on the false branch.
4. **Membership.** `k in d` records that `k` is a key of `d`, for a name or a string literal `k` and a name or `obj.f` `d`. Under that fact, `d.get(k)` (one argument, or a `None` default) has the facts of `d[k]`: unknown nullability, so a warning rather than an error. The fact is dropped when `d` or `k` is rebound and after any statement or test that calls anything other than `.get`, or has a `del` or a walrus, since a call may mutate `d`. The same holds on entry to a loop that has one, at the start of a `try` handler whose body has one, and after a `match` statement that has one in a guard or a body, at the start of a `finally` whose `try` has one in any handler or `else`, and after a definition whose decorator, default or class body has one. A test such as `k in d and f()` gives no membership fact.

## Affected users and systems
- Anyone who runs CGV on a Python codebase: fewer false non-null errors.
- `cgv/src/flow.rs`, `cgv/src/value_analysis.rs` and `cgv/src/edge_discovery.rs`, with unit tests in each.
- The new fixture `cgv/test_fixtures/cg_narrowing/`, `cgv/README.md` (the narrowing paragraph and the field-read limitation), the root `JOURNAL.md` and `docs/TASKS.md`.

## Constraints
- No change to the Lean checker, the proofs, `BehaviorModel.lean` or `protected-statements.txt`. The change is Tier 2. Its spec is `cgv/test_fixtures/cg_narrowing/expected.json` with the unit tests.
- Patterns 2 and 4 turn an error into a warning, never into silence. A value that may be None in a run the project cannot see keeps a warning.
- Every existing fixture keeps its verdict, and `scripts/bench.py run --compare bench/baseline.json` reports no lost true positive.

## Open questions
None. Two choices are settled here, and the pull request names them as tradeoffs.

- Pattern 2 assumes the project holds every caller of an eligible function. A library function called from outside the project can still receive an unguarded argument. The rule therefore lowers the error to a warning rather than to non-null, and the eligibility conditions exclude the usual ways a function reaches unseen callers (a reference as a value, a decorator, a dunder method, a framework base class). Framework dispatch by name on a class whose bases are all project classes is not yet reached. The blocking property is that the extractor reads no framework code, and the open question is which frameworks call methods of plain project classes by name.
- Pattern 4 drops the membership fact at any call, though most calls do not mutate the dict. A call can mutate `d` through an alias that the walk cannot see, so precision gives way to soundness here.
