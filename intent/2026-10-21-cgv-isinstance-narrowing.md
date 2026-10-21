# Intent: `isinstance(v, T)` narrows `v` to non-None

Task: CG-1.14. Governing roadmap item: CG-1. Issue: #5.

## Problem statement
Since CG-1.8 (#85) a parameter typed `object` may hold None. CGV narrows a name to non-None only after `is None` / `is not None` tests, truthiness tests, membership in a literal display and dereferences. It does not read `isinstance`. So this function, which never writes None, gives an error and the run exits 1:

```python
def store(value: object) -> Entry:
    if isinstance(value, str):
        return Entry.objects.create(text=value)
    ...
```

Reproduced on `origin/main` 4b84581 with the new fixture `cgv/test_fixtures/cg_isinstance_narrowing/`. Every one of the ten functions in its `ok.py` gives a false error.

## Proposed outcome
The data shape is one set of call sites, `Exits::excludes_none` in `cgv/src/flow.rs`. It holds the start offset of each call in a function body that is true only when its first argument is not None. `exits::function_exits` fills it, because only it can see what a name resolves to. `flow` reads the set and does no resolution itself, as for the other members of `Exits`. `walk_block` copies the set into the `Narrowed` state it walks with, so `flow::positive` sees it both in statement tests and in the expression contexts of `value_analysis` (`and`, `or`, `a if c else b`).

A call `isinstance(s, spec)` is in the set when all of these hold:

- `isinstance` is the builtin. The function does not bind the name (as a parameter, an assignment, an import or any other binding), and its module does not define, import or rebind it and has no star import. This is the rule `exits` already uses for the builtins `exit` and `quit`.
- The call has exactly two positional arguments, no keyword and no `*` or `**` argument.
- `spec` is a builtin class whose instances are never None (`str`, `bytes`, `bytearray`, `int`, `float`, `complex`, `bool`, `list`, `tuple`, `dict`, `set`, `frozenset`), a non-empty tuple of such specs, or a `|` union of them. Each class name must be the builtin by the same rule as `isinstance`.
- The call is not inside a `lambda`, whose parameters may bind either name.

Where such a call is the test, `flow::positive` narrows the subject `s`, which is a name or an attribute chain (`a.b.c`, read with `attr_name`). `flow::negative` narrows nothing, since `isinstance` being false says nothing about None. Every other form follows from the existing rules for `not`, `and`, `or`, `assert`, `while`, `elif`, `else` and the conditional expression. A chained subject is the same flow name as for `a.b.c is not None`, so the narrowing ends in the same way: a write to the chain or to a link of it, and a rebinding of the root. At a call site it also counts as a caller's guard, which CG-1.28's rules end when a proper prefix is handed off or written.

The spec is `cgv/test_fixtures/cg_isinstance_narrowing/expected.json`, with the unit tests in `cgv/src/flow.rs` and `cgv/src/exits.rs`. `ok.py` holds ten narrowed writes that must give no error. `bug.py`, `shadowed.py` and `shadowed_type.py` hold twelve writes that must stay errors: the negated test, a tuple with `type(None)`, `object`, `str | None`, an `or` with another condition, a parameter named `str`, a local `isinstance`, a test on another name, a rebinding after the test, a test on the link instead of the field, a module that defines `isinstance`, and a module that binds `str`. `unreached.py` pins two cases that are not yet reached: a project class, which keeps a false error, and a call that is handed the chain's link after the test, which misses an error as `is not None` does (both below).

## Affected users and systems
- People who run `crosscheck-contracts contracts check`. A write narrowed by `isinstance` with a builtin class no longer gives a false error. No error becomes a warning or silence in any other case.
- `cgv/src/flow.rs` (`Exits`, `Narrowed`, `positive`, `negative`), `cgv/src/exits.rs` (the resolution), the two callers of `positive` and `negative` in `cgv/src/value_analysis.rs`, and `cgv/README.md`. No Lean file changes, so `protected-statements.txt` stays the same.

## Constraints
- Tier 2: a behavioural change to the extractor, with no protected path.
- The change removes errors only where None cannot hold. Where the rule is unsure, the write keeps its error.
- Every existing fixture keeps its verdict, and `scripts/bench.py run --compare bench/baseline.json` reports no lost true positive.

## Decided without the maintainer
The coordinating agent recorded that the maintainer granted full autonomy for this run in chat on 2026-10-21 and was away. This file records that authorization; it does not constitute it. Each question was decided with the most conservative option. The maintainer can reverse any of them in review.

- **Which classes narrow.** Only the twelve builtin classes above. A project class is not yet reached: it may be a `typing.Protocol` marked `runtime_checkable`, an ABC with `__subclasshook__` or a `register(type(None))` call, or have a metaclass that defines `__instancecheck__`, and then `isinstance(None, C)` can be true. A library class (`decimal.Decimal`, `datetime.date`, Django models) is not yet reached for the same reason, and `object` and `collections.abc.Hashable` admit None. Those writes keep their error. The maintainer could ask for project classes whose MRO the index resolves fully, with no metaclass keyword, no Protocol or ABC base and no `register` call.
- **`builtins.isinstance` and `from builtins import isinstance`.** Not read. Only the bare, unbound name counts. A module that monkeypatches `builtins.isinstance` is not detected, as for every other builtin CGV reads.
- **A hand-off after the test.** Within one function, `if isinstance(p.a.f, str): clear(p.a)` keeps the narrowing of `p.a.f`, exactly as `if p.a.f is not None:` does today. The README already records that gap as not yet reached: the extractor has no heap or alias analysis. Ending only the `isinstance` narrowing at a hand-off would make two equivalent tests disagree, and it would not close the gap for `is not None`. The fixture pins the missed error in `unreached.py`, so the fix of the gap must update it. The maintainer could ask for a narrowing within a function to end at a hand-off of a prefix, for every test form at once.
- **Comprehension variables.** A narrowing on a comprehension variable is dropped, as for every other test (`Ctx::narrow` skips shadowed names). A comprehension that binds `str` or `isinstance` anywhere in the function makes the name local, so no call in the function narrows.

## Open questions
None. The decisions above close the questions in the task row.

Not yet reached: `isinstance` with a project or library class as the spec. The blocking property is that CGV does not prove that None is not an instance of the class. The open question is which class shapes the index can prove that for.
