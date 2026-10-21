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

Reproduced on `origin/main` 4b84581 with the new fixture `cgv/test_fixtures/cg_isinstance_narrowing/`. Every one of the ten functions of its first `ok.py` gives a false error; nine remain in `ok.py` and the tenth, `store_chained`, moved to `unreached.py`.

## Proposed outcome
The data shape is one set of call sites, `Exits::excludes_none` in `cgv/src/flow.rs`. It holds the start offset of each call in a function body that is true only when its first argument, a name, is not None. `exits::function_exits` fills it, because only it can see what a name resolves to. `flow` reads the set and does no resolution itself, as for the other members of `Exits`. `walk_block` copies the set into the `Narrowed` state it walks with, so `flow::positive` sees it both in statement tests and in the expression contexts of `value_analysis` (`and`, `or`, `a if c else b`).

A call `isinstance(v, spec)` is in the set when all of these hold:

- `isinstance` is the builtin. The function does not bind the name (as a parameter, an assignment, an import or any other binding), and its module does not define, import or rebind it and has no star import. This is the rule `exits` already uses for the builtins `exit` and `quit`.
- The call has exactly two positional arguments, no keyword and no `*` or `**` argument.
- `v` is a name the function binds (a parameter or a local) that only the function's own statements rebind (`exits::unsettled`): no walrus anywhere in the function binds it, the function does not declare it `global` or `nonlocal`, no nested function or class declares it `nonlocal`, no `lambda` body or generator expression in the function reads it, and no comprehension binds the name. The function is not module-level code: the pseudo function `<module m>` never narrows, since any function may rebind a module variable through `global`.
- `spec` is a builtin class whose instances are never None (`str`, `bytes`, `bytearray`, `int`, `float`, `complex`, `bool`, `list`, `tuple`, `dict`, `set`, `frozenset`), a non-empty tuple of such specs, or a `|` union of them. Each class name must be the builtin by the same rule as `isinstance`.
- The call is not inside a `lambda`, whose parameters may bind either name.

Where such a call is the test, and its callee is the name `isinstance` (so a call on its result, which starts at the same offset, is not taken for it), `flow::positive` returns `v`, the same name that `v is not None` returns. `flow::negative` narrows nothing for it, since `isinstance` being false says nothing about None. Every other form follows from the existing rules for `not`, `and`, `or`, `assert`, `while`, `elif`, `else` and the conditional expression, and the narrowing ends where the narrowing of `v is not None` ends: at the statements that rebind `v` (an assignment, `del`, a `for`, `with` or `except` target, an import, a `match` capture).

An attribute subject (`p.f`, `p.a.f` and deeper) is never narrowed by `isinstance`. It is not yet reached, below.

The spec is `cgv/test_fixtures/cg_isinstance_narrowing/expected.json`, with the unit tests in `cgv/src/flow.rs` and `cgv/src/exits.rs`. `ok.py` holds nine narrowed writes on a parameter that must give no error. `bug.py`, `shadowed.py` and `shadowed_type.py` hold thirty-seven writes that must stay errors (`expected.json` has 39 paths with `unreached.py`'s two). They cover the negated test, a tuple with `type(None)`, `object`, `str | None`, an `or` with another condition, a parameter named `str`, a local `isinstance`, a test on another name, a rebinding after the test, a test on the link instead of the field, a method called on the result of `isinstance`, a module that defines `isinstance` and a module that binds `str`. For a name they pin a walrus in the test, in a later operand, in an argument and in an `and` operand, a `nonlocal` rebind from a nested function, and a `lambda` and a generator that read the name after it was rebound. The writes through an attribute chain that an earlier build of this branch narrowed (an alias, a closure, a `__dict__` write, a nested body, a hand-off in the same call or expression) stay in `bug.py` as errors, so a later attempt at attribute narrowing has to keep them. `unreached.py` pins two false errors that are not yet reached: a project class, and a chained subject (`isinstance(payload.address.postcode, str)`).

## Affected users and systems
- People who run `crosscheck-contracts contracts check`. A write narrowed by `isinstance` on a name with a builtin class no longer gives a false error. No error becomes a warning or silence in any other case.
- `cgv/src/flow.rs` (`Exits`, `Narrowed`, `positive`, `negative`), `cgv/src/exits.rs` (the resolution and `unsettled`), the two callers of `positive` and `negative` in `cgv/src/value_analysis.rs`, and `cgv/README.md`. No Lean file changes, so `protected-statements.txt` stays the same.

## Constraints
- Tier 2: a behavioural change to the extractor, with no protected path.
- The change removes errors only where None cannot hold. Where the rule is unsure, the write keeps its error.
- No error that `origin/main` reports may disappear, other than the false errors of `ok.py`, bar the `frame.f_locals` case of the decisions below. This is checked per finding (path, guarantee, requirement and write site) over every fixture and every pipeline run of the bench, against a build of `origin/main`.
- Every existing fixture keeps its verdict, and `scripts/bench.py run --compare bench/baseline.json` reports no lost true positive.

## Decided without the maintainer
The coordinating agent made each decision below under the full autonomy and the authority to merge that the maintainer granted it directly in chat on 2026-10-21, while away. This file records that grant and the decisions; the grant is the chat, not this file. Each question was decided with the most conservative option. The maintainer can reverse any of them in review.

- **Which classes narrow.** Only the twelve builtin classes above. A project class is not yet reached: it may be a `typing.Protocol` marked `runtime_checkable`, an ABC with `__subclasshook__` or a `register(type(None))` call, or have a metaclass that defines `__instancecheck__`, and then `isinstance(None, C)` can be true. A library class (`decimal.Decimal`, `datetime.date`, Django models) is not yet reached for the same reason, and `object` and `collections.abc.Hashable` admit None. Those writes keep their error. The maintainer could ask for project classes whose MRO the index resolves fully, with no metaclass keyword, no Protocol or ABC base and no `register` call.
- **`builtins.isinstance` and `from builtins import isinstance`.** Not read. Only the bare, unbound name counts. A module that monkeypatches `builtins.isinstance` is not detected, as for every other builtin CGV reads.
- **Which subjects narrow (replaces the earlier decision on a hand-off after the test).** The coordinator decided this under the chat authorization above, which this file records and does not constitute. `isinstance` narrows a name only, never an attribute. Five rounds of review each found a new way an `isinstance` narrowing on an attribute chain outlives an effect in the same expression or statement and removes an error that `origin/main` reports: two parameters bound to one object and an alias made in the caller, a hand-off, `await` or walrus in the middle of an expression, a display, and a `lambda` body or generator element that runs later. Each fix added a rule and the next round found another family, so the premise that a narrowing on an attribute can be made to end at every point where code may write the field, without reading operands in evaluation order and without an alias analysis, was dropped. A name differs: a callee cannot make a local name None, and only the function's own code can rebind it. The narrowing of `v is not None` ends at the statements that rebind `v`, and `isinstance` reuses it unchanged. Where that narrowing would not end in time, `exits::unsettled` keeps `isinstance` from narrowing the name at all: a walrus, which rebinds the name between a test and a read that the walk takes as one step; a `global` or `nonlocal` name, which a call can rebind; a `nonlocal` rebind in a nested function; a read in a `lambda` body or a generator expression, which run after later statements; a comprehension variable, which `bound_names` counts as the function's own although the same name read outside may be a global; and every name of module-level code. `v is not None` keeps its own behaviour in those cases, which this change does not alter. With this rule no error that `origin/main` reports disappears, except where code writes a caller's local through `frame.f_locals` (below). That is checked per finding over every fixture and every pipeline run of the bench, and none of them exercises it. The cost is one of the ten false errors of the first `ok.py` (`store_chained`), which is now in `unreached.py`. On Python 3.13 a callee can rebind a local through the caller's `frame.f_locals` (PEP 667). The walk does not see frame access, so that write ends no narrowing of a name, of either kind: a missed error, not a false one. It is not modelled (README), and no fixture or bench run exercises it.
- **Comprehension variables.** A narrowing on a comprehension variable is dropped, as for every other test (`Ctx::narrow` skips shadowed names). A comprehension that binds `str` or `isinstance` anywhere in the function makes the name local, so no call in the function narrows.

## Open questions
The decisions above close the questions in the task row.

Not yet reached: `isinstance` with a project or library class as the spec. The blocking property is that CGV does not prove that None is not an instance of the class. The open question is which class shapes the index can prove that for.

Not yet reached: `isinstance` on an attribute (`p.f`, `p.a.f`). The blocking property is that the walk reads the operands of one expression or statement against one state and tracks no aliases, so a call, an `await` or a walrus between the test and the read, or code that holds `p` or a link under another name, can write the field unseen. The open question is whether reading operands in evaluation order, with an alias analysis, would let an attribute narrowing end at every such point.
