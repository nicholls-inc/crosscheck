# Plan: Reject `implemented_by` and `extern` on what the soundness theorems reach

Intent: `intent/2026-10-07-compiled-code-attributes.md`. Spec: `intent/2026-10-07-compiled-code-attributes-spec.md`. Task TB-1.2, issue #48, roadmap item TB-1. Tier 3: the generator, the CI workflow and the rules file are protected.

## Files, in order

1. `.assurance/protected-surface-amend/compiled-code-attributes-2026-10-07.md`. The governance note, first, so the protected-surface hook allows the edits below.
2. `cgv/prover/scripts/ProtectedStatements.lean`. Add, after `checkAxioms`:
   - `inProject env n`: true when `env.getModuleIdxFor? n` names a module under `ContractGraph`.
   - `runtimeDependencies env ci`: the constants in the type, the value with opaque values included, an inductive's constructors, and `n._unsafe_rec` when the environment has it.
   - `runtimeReach env`: a worklist from the roots of SM-7 that skips theorems and constants outside the project.
   - `checkCompiledCode env`: throws, naming each reached constant where `Compiler.implementedByAttr.getParam? env n` is `some` or `isExtern env n` holds.

   Call `checkCompiledCode` in `render` before any output. Update the header comment. Leave `reach`, `dependencies` and `inScope` unchanged, so the manifest is unchanged (SM-7a).
3. `cgv/prover/scripts/compiled-code-selftest.sh`. The five cases of SM-8. It backs up `ContractGraph/Main.lean`, restores it on exit with a `trap`, and checks both the exit code and the named constant.
4. `.github/workflows/cgv-ci.yml`. A step after the manifest step that runs the self-test from `cgv/prover`.
5. Docs: one sentence each in `.claude/rules/protected-surfaces.md` (CGV proof surfaces), `cgv/CLAUDE.md` (the generator paragraph) and `cgv/README.md` (trust model, theorem statements row).
6. `docs/TASKS.md`: TB-1.2 `done` with this intent as its record, and a new row TB-1.7 for `@[csimp]` lemmas. TB-1.5 and TB-1.6 are taken by open PR #99. Root `JOURNAL.md` entry.

## Risks

- **Reach too narrow.** A constant the binary runs but the walk misses would escape. Mitigation: the walk follows types as well as values, opaque values and `_unsafe_rec` helpers, and scopes by module, not by namespace, so private helpers and root-namespace instances count. Cases 1, 3 and 4 of SM-8 test a constant reached only through a value, an `_unsafe_rec` helper and an opaque value.
- **Reach too wide.** A false failure would block an unrelated change. Case 5 of SM-8 checks that an unreached constant passes.
- **Manifest drift.** If the new code touched `reach`, the manifest would change. SM-7a: the manifest step in CI diffs it.
- **Self-test leaves a mutated source.** The script restores `Main.lean` in a `trap` on exit, and CI runs it in a throwaway checkout.
- **Conflict with PR #99.** Both add a step to `cgv-ci.yml` after the manifest step and edit the same paragraph of `protected-surfaces.md`. Whichever merges second resolves by keeping both.

## Proof that it works

- `lake build ContractGraph ContractGraph.Main && lake env lean --run scripts/ProtectedStatements.lean | diff -u protected-statements.txt -` prints nothing (SM-7a).
- `scripts/compiled-code-selftest.sh` exits 0 (SM-8).
- Mutation: with `checkCompiledCode` removed from `render`, or the theorem roots removed, the self-test fails on every reject case. With the value arm removed it fails on cases 1, 3 and 4, with the `_unsafe_rec` arm removed on case 3, with opaque values excluded on case 4, with the `extern` test removed on case 2, and with the `implemented_by` test removed on cases 1, 3 and 4. With `inProject` true for every module it fails on case 5, and the error lists the library constants the walk then reaches.
