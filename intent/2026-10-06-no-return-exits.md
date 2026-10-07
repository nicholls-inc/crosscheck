# Intent: A call that never returns, and a match over every enum member, end the flow

Task: CG-1.11. Governing roadmap item: CG-1. Issue: #5.

## Problem statement
CGV treats a function body as falling through to an implicit `return None` when its last statement is a call that never returns, or a `match` that covers every member of an enum but has no wildcard. Against a non-Optional return annotation, that fall-through is a non-null error, and it is false. The same rule decides narrowing: an `if x is None: sys.exit(1)` branch does not count as an exit, so `x` stays possibly None after it.

On the codebase that `cgv/docs/evaluation/real-codebase-evaluation-2026-09.md` measured, this cause produced 2 of the 20 triaged non-null errors. The labelled benchmark has one case of it: `describe` in `cgv/bench/corpus/labelled/app/false_positives.py`, labelled `no-return`, whose wildcard case is `assert_never(kind)`.

The cause is in `cgv/src/flow.rs`. `terminates` (behind `falls_through`) and `always_exits` (behind narrowing in `walk`) look only at `return`, `raise`, `continue` and `break`, and `match_is_exhaustive` accepts only an unguarded wildcard or capture case. Both functions see one body and nothing else, so they cannot tell what a call resolves to or what members an enum has.

## Proposed outcome
The project index decides, once per function, which of its statements never complete, and `flow.rs` reads that decision. The data shape is a set of statement offsets per function, kept with the `FunctionInfo`. It has two kinds of entry.

1. **A call that never returns.** An expression statement whose value is a call, or an `await` of a call, ends the flow when the callee resolves to one of these:
   - `typing.assert_never`, `typing_extensions.assert_never`, `sys.exit`, `os._exit`, `os.abort`, reached through an import of a module that is not a project module;
   - the builtins `exit` and `quit`, when the module neither defines nor imports the name;
   - a project function whose return annotation is `NoReturn` or `Never` from `typing` or `typing_extensions`, with no wrapping decorator, not an overload stub, and not a generator. A plain call of such a function needs it synchronous. An `await` of such a call needs it `async`, because a plain call of an `async def` returns a coroutine at once.

   A name that the calling function binds itself (a parameter, an assignment, a local `def` or import) never counts, so a local `exit = ...` hides the builtin.
2. **A match over every member of an enum.** A `match` statement ends the same way as one with a wildcard case when all of these hold:
   - its subject is a bare name that is a parameter of the function, annotated with exactly one project enum class `E` (not `Optional[E]`, not a union), and the function never rebinds the name;
   - `E` is an `enum.Enum`, `IntEnum` or `StrEnum` subclass, or a Django `TextChoices` or `IntegerChoices` subclass. A `Flag` or `IntFlag` is not, since a combined value matches no member case;
   - `E`'s body binds members only by simple assignment, and defines no `__eq__` and no `_ignore_`. Any other class-level binding except `def`, a docstring and `pass` makes the member list uncertain, so the match is not treated as exhaustive;
   - each member name of `E` is matched by an unguarded case whose pattern is a value pattern `E.NAME`, an or-pattern of them, or an `as` pattern around one.

Everything else stays as it is. An unrecognised call is assumed to return, and a match that the rule does not cover is assumed to fall through. Both assumptions can only add errors, never hide one.

A new fixture, `cgv/test_fixtures/no_return_exits/`, pins the behaviour, and its `expected.json` is the spec. Its `ok.py` covers each exit above, a narrowing after `if x is None: sys.exit(1)`, and an enum match with an or-pattern and an `auto()` member. Its `models.py` keeps the errors that must stay: a match that misses one enum member, a `Flag` subject, an `Optional[E]` subject, a decorated `NoReturn` function, a plain call of an `async` `NoReturn` function, and a local name `exit` that shadows the builtin. The `no-return` case in the labelled benchmark stops being reported.

## Affected users and systems
- People who run `crosscheck-contracts contracts check` on code that ends a branch with `assert_never`, `sys.exit` or its own `NoReturn` helper, or matches over every member of an enum. They see fewer false non-null errors, and more narrowing after such a branch.
- `cgv/src/flow.rs`, the index build that fills `FunctionInfo`, and a new fixture. `FunctionInfo` gains whether the function is `async`. No Lean file changes, so `protected-statements.txt` stays the same.

## Constraints
- The rule rests on the type checker's semantics, as parameter preconditions already do. A `NoReturn` annotation is trusted the way a parameter annotation is, and a subject annotated `E` is assumed to hold a member of `E`. The README's trust model already states that contracts are relative to a type-correct program, so `BehaviorModel.lean` needs no new rule.
- Some cases are not yet reached:
  - A method called through an instance (`self.fail()`). The blocking property is dispatch: an override in a subclass may return. The open question is whether the index's dispatch set (`MAX_DISPATCH`) is closed enough to require every target to be `NoReturn`.
  - A match whose subject is an attribute, or a local variable with an annotation or a known enum value. The blocking property is that the extractor tracks no type for those names. The open question is how far to take local type inference in the extractor.
  - An enum from outside the project. The blocking property is that the index has no member list for it. The open question is whether to read stub files.
  - A third-party `NoReturn` function other than the five named above, for the same reason.
- The tier is 2: a behavioural change to the extractor, with no protected path.

## Open questions
None.
