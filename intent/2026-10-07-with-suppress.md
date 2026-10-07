# Intent: A context manager may suppress what a `with` body raises

Task: CG-1.24. Governing roadmap item: CG-1. Issue: #5.

## Problem statement
A context manager whose `__exit__` returns true suppresses the exception raised in its body, and control continues after the `with`. `contextlib.suppress(E)` and `pytest.raises(E)` do this. CGV does not model it, so it reports exit 0 on code that returns None from a function annotated `-> str`.

The cause is in `cgv/src/flow.rs`:

- `terminates` (behind `falls_through`) counts a `with` whose body ends in `raise` as an end of the flow. So `with suppress(ValueError): raise ValueError(k)` at the end of a function hides its implicit `return None`.
- The `With` arm of `walk` keeps what the body narrowed, unless the body holds a call that never returns or an enum-exhaustive `match` (CG-1.11). So `with suppress(AssertionError): assert x is not None`, `with suppress(ValueError): if x is None: raise ValueError(...)` and `with suppress(AttributeError): x.upper()` all leave `x` non-None after the `with`, and a later `return x` passes.

Before this change, the binary built from `origin/main` (4f0c1e0) exits 0 on `cgv/test_fixtures/with_suppress/bad.py`, which returns None in each of its six functions annotated `-> str`. Its only finding is a warning.

## Proposed outcome
Any context manager may suppress any exception raised in its body. The extractor cannot tell which managers do, so it assumes every one may.

1. **Termination.** A `with` ends the flow only when its body ends in `return` on every path. A `raise`, a call that never returns, and a `while True:` loop with no `break` do not count inside a `with` body, at any depth, since each of them leaves the body only by an exception. Outside a `with`, nothing changes.
2. **Narrowing.** What a `with` body narrows does not survive the `with`. After it, the state is the state after the first item, minus the names the `with` binds, with the effects of the whole statement applied. The first item's expression runs outside every manager, so what it narrows holds; a later item's expression runs inside the earlier managers (`with suppress(AttributeError), x.open(): pass`), so what it narrows does not survive. The body's effects apply (a call in the body may have run before the suppressed exception, so `k in d` does not survive `d.pop(k)`). This replaces the CG-1.11 rule, which dropped the narrowing only for a body that holds a marked exit.

The data shape is a two-value enum, `Raised`, passed through `terminates`: an exception either propagates or may be suppressed. `Exits::marks_within` has no caller left and is deleted.

`cgv/test_fixtures/with_suppress/` pins the behaviour, and its `expected.json` is the spec. `bad.py` holds six functions that return None, each now an error: a `raise` under `suppress`, a `raise` under `pytest.raises`, the three narrowing forms above, and a later `with` item that dereferences the name. `ok.py` holds four that must stay clean: a `with lock:` body that returns on every path, a guard before a `with` that survives it, and a `raise` after a `with`, and a first `with` item that dereferences the name, whose narrowing survives.

## Benchmark corpus
The row asks to measure the corpus first, since the change alters verdicts on code with no `NoReturn` call. The public corpus (`cgv/bench/corpus/`) holds no `with` statement. `scripts/bench.py run --out` gives the same result before and after the change, and the same as `cgv/bench/baseline.json` apart from `tool_commit`, so the baseline is not regenerated. No fixture under `cgv/test_fixtures/` held a `with` before this change either, and all 102 fixtures pass after it. The change to verdicts on a real codebase is not measured here, since the codebase of the real-codebase evaluation is private.

## Affected users and systems
- People who run `crosscheck-contracts contracts check` on code with `with` statements. They see an error where a suppressed exception lets None reach a non-Optional return or a non-null field. They also see errors where a guard inside a `with` body narrowed a name that is used after it, even when the manager suppresses nothing (`with lock: if x is None: x = default`, then `return x`). That second kind is a precision cost, and CG-1.47 takes it.
- `cgv/src/flow.rs` and a new fixture. No Lean file changes, so `protected-statements.txt` stays the same.

## Constraints
- The change can only add errors, never hide one: it removes facts and ends of flow, and adds none.
- Some cases are not yet reached:
  - An exception raised by a statement of a `with` body before its final `return`, or by the value of that `return`, under a manager that suppresses it (`with suppress(KeyError): return d[k]`). The function then falls through to `return None`, and CGV still reports that it does not. The blocking property is that the extractor does not know which managers suppress. Treating every `with` that ends in `return` as falling through would close it, but it would report an error on every `with open(p) as f: return f.read()` in a function annotated `-> str`. The open question is whether to recognise a closed list of managers that never suppress (a lock, a file, `tempfile`, `transaction.atomic`) and treat every other `with` as falling through. CG-1.47 takes it.
  - An exception raised by the expression of a later `with` item, or by a `raise` in a nested `try` handler, before the body's final `return` (`with suppress(E), g() as y: return y`). Same blocking property, same open question, same row.
  - The same closed list would let a `with lock:` body keep its narrowing and its `raise` count as an end of the flow. Same blocking property, same open question, same row.
- The tier is 2: a behavioural change to the extractor, with no protected path.

## Open questions
None beyond the one named under Constraints, which CG-1.47 tracks.
