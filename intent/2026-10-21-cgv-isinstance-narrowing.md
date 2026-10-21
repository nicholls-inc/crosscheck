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

Where such a call is the test, and its callee is the name `isinstance` (so a call on its result, which starts at the same offset, is not taken for it), `flow::positive` narrows the subject `s`, which is a name or an attribute chain (`a.b.c`, read with `attr_name`). `flow::negative` narrows nothing, since `isinstance` being false says nothing about None. Every other form follows from the existing rules for `not`, `and`, `or`, `assert`, `while`, `elif`, `else` and the conditional expression. A chained subject is the same flow name as for `a.b.c is not None`, so the narrowing ends in the same way at a write to the chain or to a link of it, and at a rebinding of the root.

An attribute subject (`p.f`, `p.a.f` and deeper) also ends where CG-1.28 (#132) ends a caller's guard on it. `flow::positive` returns it as a `Fact` with `held` set, and `Narrowed` keeps the held names in a set of their own. `Narrowed::apply` drops a held name at the first node whose `Effects` hand it off, by `Effects::hands_off`: the node suspends (an `await`, a `super()` call), or it reaches the root `p` (`clear(p)`, `p.m()`), or, for a chain, it hands an attribute of `p` or a proper prefix of the chain (`clear(p.a)`, `x = p.a.b` for `p.a.b.f`). These are the `Effects` fields and the prefix rule (`is_link_of`, shared with `Narrowed::link_handed`) that `Scope::field_read_facts` reads, through `untouched`, `links_untouched` and `link_handed`, for a caller's guard. A caller's guard counts from the function's entry, so the narrowing starts only where that guard would hold at the test: `flow::positive` takes the state before the test and `Narrowed::guard_intact` requires that the root is a parameter that is not rebound (`Def::Param` only), that no code was handed it (`untouched`), that for a chain no attribute of the root and no proper prefix of the chain was handed (`links_untouched`, `link_handed`), and that no `root.__dict__` was read (`Effects::exposes`, `Narrowed::exposed`). An alias or a closure made before the test (`q = p`, `a = p.a`, `f = lambda: clear(p.a)`, `t = p.__dict__`) therefore ends the narrowing before it starts. From the test on, `hands_off` ends it, as before, and a read of `root.__dict__` counts as a hand-off. A walrus in the test that binds the root of an `isinstance` fact, or a plain subject, drops that fact (`Effects::rebinds`, `Fact::by_isinstance`); an `is not None` fact is not touched by it. A name that another test narrows as well (`is not None`, truthiness) is not held, so `is not None` keeps today's behaviour, which the README records as not yet reached.

The spec is `cgv/test_fixtures/cg_isinstance_narrowing/expected.json`, with the unit tests in `cgv/src/flow.rs` and `cgv/src/exits.rs`. `ok.py` holds ten narrowed writes that must give no error. `bug.py`, `shadowed.py` and `shadowed_type.py` hold fourteen writes that must stay errors: the negated test, a tuple with `type(None)`, `object`, `str | None`, an `or` with another condition, a parameter named `str`, a local `isinstance`, a test on another name, a rebinding after the test, a test on the link instead of the field, a method called on the result of `isinstance` (`isinstance(v, str).__round__(w)`, a call that starts at the same offset), a call that is handed the chain's link after the test (`clear(payload.address)`), a module that defines `isinstance`, and a module that binds `str`. `unreached.py` pins one case that is not yet reached: a project class, which keeps a false error (below).

## Affected users and systems
- People who run `crosscheck-contracts contracts check`. A write narrowed by `isinstance` with a builtin class no longer gives a false error. No error becomes a warning or silence in any other case.
- `cgv/src/flow.rs` (`Exits`, `Narrowed`, `positive`, `negative`), `cgv/src/exits.rs` (the resolution), the two callers of `positive` and `negative` in `cgv/src/value_analysis.rs`, and `cgv/README.md`. No Lean file changes, so `protected-statements.txt` stays the same.

## Constraints
- Tier 2: a behavioural change to the extractor, with no protected path.
- The change removes errors only where None cannot hold. Where the rule is unsure, the write keeps its error.
- No error that `origin/main` reports may disappear, other than the false errors of `ok.py`. This is checked per finding (path, guarantee, requirement and write site) over every fixture and every pipeline run of the bench, against a build of `origin/main`.
- Every existing fixture keeps its verdict, and `scripts/bench.py run --compare bench/baseline.json` reports no lost true positive.

## Decided without the maintainer
The coordinating agent recorded that the maintainer granted it full autonomy and the authority to merge for this run, in chat on 2026-10-21, and was away. This file records that authorization; it does not constitute it. Each question was decided with the most conservative option. The maintainer can reverse any of them in review.

- **Which classes narrow.** Only the twelve builtin classes above. A project class is not yet reached: it may be a `typing.Protocol` marked `runtime_checkable`, an ABC with `__subclasshook__` or a `register(type(None))` call, or have a metaclass that defines `__instancecheck__`, and then `isinstance(None, C)` can be true. A library class (`decimal.Decimal`, `datetime.date`, Django models) is not yet reached for the same reason, and `object` and `collections.abc.Hashable` admit None. Those writes keep their error. The maintainer could ask for project classes whose MRO the index resolves fully, with no metaclass keyword, no Protocol or ABC base and no `register` call.
- **`builtins.isinstance` and `from builtins import isinstance`.** Not read. Only the bare, unbound name counts. A module that monkeypatches `builtins.isinstance` is not detected, as for every other builtin CGV reads.
- **A hand-off after the test.** The coordinator decided this, under the chat authorization above, which this file records and does not constitute. The first build kept the narrowing of `p.a.f` after `if isinstance(p.a.f, str): clear(p.a)`, as `if p.a.f is not None:` does, and so lost an error that `origin/main` reports (`store_chain_handed`). The rule for this change is that no error `origin/main` reports may disappear. So an `isinstance` narrowing on an attribute goes through CG-1.28's caller-guard conditions and ends at a hand-off of the root, of an attribute of the root (for a chain) or of a proper prefix of the chain, and at a suspension or a `super()` call. `store_chain_handed` is an expected error in `bug.py`. The conditions are as coarse as CG-1.28's, counted from the function's entry up to the test and from the test on: handing any attribute `p.x` ends the narrowing of every chain of `p`, a source of false errors, not of missed ones. They are no coarser than CG-1.28's about aliases: two parameters that hold the same object, and an alias made in the caller, are not yet reached for the guard (README), so they are not for `isinstance` either; a write through such an alias keeps the narrowing and removes an error that `origin/main` reports. The first review found the pre-test case (an alias, a closure or a `__dict__` read made before the test) and a walrus in the test, and both now end the narrowing; fixtures `store_alias_before`, `store_link_alias_before`, `store_param_alias_before`, `store_closure_before`, `store_walrus_in_test`, `store_dict_write` and `store_dict_alias_before` pin them. `is not None` and truthiness narrowing within one function are unchanged and keep the gap the README records. The maintainer could ask for that gap to be closed for every test form at once, which would add errors that `origin/main` does not report.
- **Comprehension variables.** A narrowing on a comprehension variable is dropped, as for every other test (`Ctx::narrow` skips shadowed names). A comprehension that binds `str` or `isinstance` anywhere in the function makes the name local, so no call in the function narrows.

## Open questions
None. The decisions above close the questions in the task row.

Not yet reached: `isinstance` with a project or library class as the spec. The blocking property is that CGV does not prove that None is not an instance of the class. The open question is which class shapes the index can prove that for.
