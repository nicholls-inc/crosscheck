## Protected-Surface Amendment

**Target file(s):** `cgv/prover/scripts/ProtectedStatements.lean`, `.github/workflows/cgv-ci.yml`, `.claude/rules/protected-surfaces.md`
**Class:** CGV proof surface (the manifest generator); A (CI enforcement; harness rules)
**Matched rule:** `cgv/prover/scripts/ProtectedStatements.lean`, `.github/workflows/**`, `.claude/rules/**`
**Date:** 2026-10-07

### Change Description

1. `cgv/prover/scripts/ProtectedStatements.lean`: a new check, `checkCompiledCode`, runs before any output. It walks from the constants in every protected theorem's statement and from the three protected definitions, through the types, values, constructors and `_unsafe_rec` helpers of every non-theorem constant defined in a `ContractGraph` module, and fails if a reached constant carries `@[implemented_by]` or `@[extern]` (SM-7). The manifest output is unchanged (SM-7a).
2. `.github/workflows/cgv-ci.yml`: a new step after the manifest step runs `scripts/compiled-code-selftest.sh` (SM-8).
3. `.claude/rules/protected-surfaces.md`: one sentence in "CGV proof surfaces" says the generator also fails on either attribute in the reached code.

### Rationale

Task TB-1.2, issue #48. The `runChecker_*` theorems are proved about the definitions, and the binary runs the compiled code. Either attribute replaces the compiled code of a definition without adding an axiom, so the SM-6 axiom check and the manifest cannot see it. Nothing in `ContractGraph/` uses either attribute today. Intent: `intent/2026-10-07-compiled-code-attributes.md`. Spec: `intent/2026-10-07-compiled-code-attributes-spec.md`. Plan: `intent/2026-10-07-compiled-code-attributes-plan.md`.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (item TB-1)
- **Title:** Shrink the trusted base, and measure what remains
- **Scope coverage:** TB-1 lists issue #48. Task TB-1.2 in `docs/TASKS.md` is this change.

### Authority

- **Authoriser:** harry-nicholls. The merge is the approval.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `cgv/prover/scripts/ProtectedStatements.lean` | header comment; new `inProject`, `runtimeDependencies`, `runtimeReach`, `checkCompiledCode`; `render` | check added, output unchanged |
| 2 | `.github/workflows/cgv-ci.yml` | new step after the manifest step | added |
| 3 | `.claude/rules/protected-surfaces.md` | CGV proof surfaces | one sentence added |

### Test / Coverage Impact

- `cgv/prover/scripts/compiled-code-selftest.sh` (SM-8) puts `@[implemented_by]` on a constant reached only through `runChecker`'s value, `@[extern]` on a constant a theorem statement names, and `@[implemented_by]` on definitions reached only through a `partial def`'s `_unsafe_rec` helper and through an `opaque` value. It expects the generator to fail on each and name the constant. It puts `@[implemented_by]` on an unreached constant and expects success. Each arm of the walk was removed in turn, and a case failed each time.
- The committed manifest `cgv/prover/protected-statements.txt` is unchanged, and CI's manifest step checks that.
- No theorem statement, no definition, and no invariant changes. Exit 0 promises the same for the definitions. For the binary it now promises what the theorem says for every reached project constant.

### Review Checklist

- [x] Rationale is anchored to issue #48 and a reproduction by the self-test.
- [x] Authoriser is a named human.
- [x] TB-1 lists issue #48.
- [x] The diff plan names every changed protected file.
- [x] No check is weakened: the manifest and the axiom check are unchanged, and the new check only adds a failure.
- [ ] REQUIRES HUMAN VERIFICATION: the maintainer accepts that library code (Lean core, Std, `leansqlite`) and `@[csimp]` lemmas stay not yet reached, as the spec records (TB-1.7 for `@[csimp]`).
