# Spec: The compiled code of the protected theorems is the code they are about

Intent: `intent/2026-10-07-compiled-code-attributes.md`. It extends the CGV statement manifest requirements SM-1 to SM-6 in `intent/2026-09-29-deterministic-evidence-spec.md`. Governing roadmap item: TB-1.

## Requirements

- **SM-7.** `cgv/prover/scripts/ProtectedStatements.lean` fails with a non-zero exit, and prints the offending names, if a *runtime-reached* constant carries `@[implemented_by]` or `@[extern]`. A runtime-reached constant is one that:
  - is defined in a module whose name starts with `ContractGraph`, whatever its own namespace, so private constants and instances count; and
  - is reached from a root by following, through constants that are not theorems: the constants used in the type and the value (an opaque's value included), an inductive type's constructors, and a definition's `_unsafe_rec` helper, which the compiler runs in place of a recursive definition.

  The roots are the constants used in the statement of every protected theorem, and the protected definitions `constraintImplies`, `IsDataPath` and `stepwiseSound`. The walk does not enter a theorem, because proofs are erased from compiled code. It does not continue into a constant outside the project, because such a constant cannot refer back to project code.
- **SM-7a.** SM-7 adds no output. On the current sources the manifest is byte-identical to the committed `protected-statements.txt`.
- **SM-8.** `cgv/prover/scripts/compiled-code-selftest.sh` checks SM-7 against the real sources. For each case it edits `ContractGraph/Main.lean`, rebuilds, runs the generator, and restores the file:
  1. `@[implemented_by]` on `lastN`, which only `runChecker`'s value reaches: the generator must fail and name `ContractGraph.lastN`.
  2. `@[extern]` on `defaultMaxStates`, which `runChecker_exitCode_eq_zero_iff`'s statement names: the generator must fail and name `ContractGraph.defaultMaxStates`.
  3. `defaultMaxStates` calls a recursive `partial def`, whose body calls a definition with `@[implemented_by]`. The kernel sees an opaque constant, and only the `_unsafe_rec` helper reaches the definition: the generator must fail and name it.
  4. `defaultMaxStates` calls an `opaque` constant whose value is a definition with `@[implemented_by]`: the generator must fail and name it.
  5. `@[implemented_by]` on `outputToJson`, which no root reaches: the generator must succeed.

  The script exits non-zero if any case gives the other result, or if an edit matches nothing. CGV CI runs it after the manifest step.

## Not yet reached

- **Library code.** Lean core, Std and `leansqlite` use `implemented_by` and `extern` widely, for example for `Array` and `String` operations. On this branch the reached project code calls 19 such constants directly, and reaches 103 in all when the walk also follows the libraries' own code. They are part of the trusted base with the compiler itself. The property that blocks a check is a proof that each library implementation agrees with its definition, which the toolchain does not ship. The open question is whether to pin an audited list of the library pairs that the reached code uses, and fail when it grows.
- **`@[csimp]` lemmas.** A `@[csimp]` theorem `@f = @g` makes the compiler run `g` for `f`. It is proved, so it is sound when its proof is. SM-6 checks the axioms of protected theorems only, so a project `@[csimp]` lemma closed by `sorry` would change the compiled code of a reached constant without failing CI. The property that blocks it is an axiom check on the `@[csimp]` lemmas that rewrite reached constants. The open question is whether to check only those lemmas or every `@[csimp]` lemma in the project. TB-1.7 in `docs/TASKS.md` holds the work.
- **Code that no theorem reaches.** `main` (including how the arguments become an exit code), the translation from SQLite and the JSON output are not reached, because no theorem is about them. So an `implemented_by` on `main` would make the binary exit 0 without running `runChecker`, and the generator would pass. Extraction and translation are already untrusted in the trust model. The property that blocks a check is a theorem about `main`'s mapping from arguments and `runChecker`'s result to the exit code. The open question is whether to prove that theorem, or to check at fixture level that the binary's exit code equals `runChecker`'s.
