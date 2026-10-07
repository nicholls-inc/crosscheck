# Intent: A call through a subclass, and an enum's bases and decorators, are read as Python reads them

Task: CG-1.27. Governing roadmap item: CG-1. Issue: #5.

## Problem statement
CG-1.11 made a call that never returns, and a `match` over every member of an enum, end the flow. The pr-swarm review of CG-1.11 (round 2) found seven forms where those rules mark a statement as an exit although Python runs past it. Each one hides a real non-null error: the function falls through to an implicit `return None` against a non-Optional return annotation, and `crosscheck-contracts contracts check` exits 0.

Method resolution, in `ProjectIndex::method_rec` in `cgv/src/resolve.rs`, which `Sub.s(x)` reaches through `resolve_expr`:
1. `class Sub(Mixin, Base)` where the index cannot read `Mixin`. The search skips `Mixin` and finds `Base.s`, annotated `NoReturn`. `Mixin` may define `s`.
2. `class Sub(Base): s = staticmethod(print)`. The search reads only `methods`, the `def`s of `Sub`, so it skips the assignment and finds `Base.s`.
3. `class Diamond(Left, Right)` where `Left` and `Right` both subclass `Base`, and `Right` overrides `s`. The search is depth-first, so it finds `Base.s` through `Left`. Python's MRO is `Diamond, Left, Right, Base`, so it finds `Right.s`.
4. A class whose body defines `def s(...) -> NoReturn` and then binds `s = staticmethod(print)`. The index keeps the `def`.

Enum bases, in `enum_class` in `cgv/src/exits.rs`:
5. `from other_lib import Enum` then `class Color(Enum)`. The base is accepted by its last dotted segment, `Enum`, so a match over `Color.RED` and `Color.GREEN` counts as exhaustive although `other_lib.Enum` may be any class.

Enum member detection, in `enum_member_names` and `is_member_decorator` in `cgv/src/resolve.rs`:
6. `from enum import member as m` then `@m def LIGHT(self)`. Only a decorator named `member` makes a method a member, so `LIGHT` is not counted and a match over the other members counts as exhaustive.
7. `@const def HIGH(self)` where `const` returns `3`. A decorator that returns a value that is not a descriptor makes the name a member, here with value `3`.

A run of the CG-1.11 tool on one fixture with all seven exits 0. Each form also exits 0 on its own. For forms 2, 3, 4, 6 and 7, Python 3.14 returns `None` from the function.

## Proposed outcome
Each rule reads what Python reads, and returns no exit when it cannot be sure. An uncertain result can only add errors, never hide one.

- **Method resolution.** A new `ProjectIndex::method_by_mro(class, name)` computes the class's C3 linearisation and returns the method of the first class in it whose body binds `name`. It returns `None` when a class on the way has a base that is not a project class or the builtin `object` (a bare name its module does not bind), when the bases have no consistent order, or when the body that binds `name` binds it other than by one `def`. A nested class's bases are read in the enclosing class bodies, so a base whose head name an enclosing body may bind (or an enclosing class the index lacks) also gives `None`. `body_bindings` counts a `case` capture, a walrus target and a `type` statement as bindings, and reads decorators and defaults of a `def`, but not the body of a `def`, a `lambda` or a nested `class`. `ClassInfo` gains `body_rebound`, the names its body binds more than once. `is_exit_call` in `cgv/src/exits.rs` checks that a call `K.s(...)` whose prefix resolves to a class names the method `method_by_mro` returns. `method` and `method_rec` stay as they are: they serve edge discovery and value analysis, where a result of `None` would drop edges instead of adding errors.
- **Enum bases.** `enum_class` resolves each base through its module's imports to a `(module, name)` pair and accepts only `enum.Enum`, `enum.IntEnum`, `enum.StrEnum`, `TextChoices`, `IntegerChoices` and `Choices` from `django.db.models` or `django.db.models.enums`, and the builtins `str` and `int`. The head name of a base must be neither defined nor rebound by the module, nor bound by an enclosing class body. A builtin must not be reachable through a star import. The import must not name a module that a project module may shadow: a module whose path equals the import's path or its first segment, or ends with either after a dot. `ClassInfo::with_body` keeps its last-segment rule for `enum_kind`, because `enum_kind` also feeds the `.choices` values of Django fields, and tightening it there would drop `choices` contracts, which can hide errors. With `enum_class` resolving the bases, the last-segment rule no longer reaches an exit.
- **Enum member detection.** A method of an enum body may carry only the decorators `staticmethod`, `classmethod` and `property`, each as a bare name (`ENUM_METHOD_DECORATORS`). Any other decorator makes the member list uncertain, so the match is not treated as exhaustive. When a method of the body has a decorator, the index also requires that the module neither defines, imports nor rebinds any of the three names, that it has no star import, and that the class body does not bind them. This settles the open question of the row: the enum body allows only a fixed list of decorators, and the resolver returns an uncertain result for a method.

A new fixture, `cgv/test_fixtures/no_return_resolution/`, pins the behaviour, and its `expected.json` is the spec. `methods.py`, `fake_enum.py` and `enums.py` hold the seven forms, each a non-null error. `ok.py` holds the forms that must stay exits: a `NoReturn` static method called through its own class, through a subclass and through a grandchild with `object` among its bases, and matches over a `str` and `Enum` mixin with static, class and property methods, an `enum.IntEnum` read through the module, and a `models.TextChoices`. The labelled benchmark does not change (`scripts/bench.py run --compare bench/baseline.json` reports no change).

## Affected users and systems
- People who run `crosscheck-contracts contracts check` on code that calls a `NoReturn` method through a subclass, or matches over an enum. They see a non-null error for each of the seven forms, which exited 0 before. The forms CG-1.11 accepted otherwise stay exits.
- `cgv/src/resolve.rs` (`method_by_mro`, `mro`, `ClassInfo.body_rebound`, `enum_member_names`, `ENUM_METHOD_DECORATORS`), `cgv/src/exits.rs` (`is_exit_call`, `enum_class`, `library_name`, `ENUM_BASES`), the class loop of the index build in `cgv/src/extractor.rs`, and a new fixture. No Lean file changes, so `protected-statements.txt` stays the same.

## Constraints
- The rules still trust the type checker as CG-1.11 does, and `BehaviorModel.lean` needs no new rule.
- Some forms are not yet reached:
  - A metaclass of a class on the MRO that defines `s` as a data descriptor, a class decorator that replaces `s`, and a base's `__init_subclass__` that sets `s` on the subclass. Each makes `Sub.s` other than the method `method_by_mro` returns. The blocking property is that `ClassInfo` keeps neither a class's keywords nor its decorators, and `method_by_mro` does not look at `__init_subclass__`. The open question is whether any of these on the MRO should make the lookup uncertain, or only those the index cannot read. CG-1.48 takes them.
  - An assignment to `K.s` outside the class body. CG-1.21 takes it.
  - A write to the class namespace through `locals()` in a class body (`locals()['s'] = print`). The blocking property is that the index reads no call in a class body as a write, and the open question is whether any `locals()` call in a class body should make its names uncertain. CG-1.48 takes it.
  - A write to a base's class attribute after the class (`Base.s = print`, `setattr(K, 's', print)`). The assignment `K.s = ...` stays with CG-1.21. The blocking property is that the index reads no attribute store on a class as a rebinding of the method, and the open question is whether any such write to a class on the MRO should make the lookup uncertain. CG-1.48 takes it.
  - A base whose name the module rebinds after the definition (`Left = Other`) or binds in a `try` and an `except ImportError`. `mro` reads each base with `resolve_expr`, which does not check that the name is stable. The open question is whether a base whose name is not bound once should make the lookup uncertain. CG-1.48 takes it.
  - A `del s` in a class body. `body_bindings` does not count a `del`, so a `def s` followed by `del s` still reads as the class's method. The blocking property is the same as for the metaclass forms, and CG-1.48 takes it.
  - A nested enum class whose method decorator an enclosing class body rebinds. The check reads the module and the enum's own body, not the enclosing bodies. CG-1.48 takes it.
- The tier is 2: a behavioural change to the extractor, with no protected path.

## Open questions
The row's own open question is settled above: the enum body allows a fixed list of decorators, and method resolution returns an uncertain result that `is_exit_call` refuses. The open questions of the forms not yet reached (metaclass, class decorator, `__init_subclass__`, `del`, `locals()`, a write to the class attribute, an unstable base name and the nested enum decorator) move to CG-1.48.
