# Intent: CGV CI replays every declaration through the Lean kernel

Task: TB-1.1. Governing roadmap item: TB-1. Issue: #47.

## Problem statement
CGV CI rejects a protected theorem or definition that depends on `sorry` or on an axiom other than `propext`, `Classical.choice` and `Quot.sound` (SM-6 in `intent/2026-09-29-deterministic-evidence-spec.md`). The generator `cgv/prover/scripts/ProtectedStatements.lean` does this with `collectAxioms`, which lists the constants a proof uses and does not re-check the proof. A declaration added with the kernel check switched off reports no axioms. Added to `cgv/prover/ContractGraph/Types.lean` on this branch, with `import Lean` at its top, these lines keep `lake build` green, leave the manifest byte-identical, and the generator exits 0:

```lean
open Lean in
set_option debug.skipKernelTC true in
run_meta addDecl (.thmDecl { name := `bad, levelParams := [], type := mkConst ``False, value := mkConst ``True.intro })
theorem uses : 1 = 2 := (bad).elim
#print axioms uses   -- 'uses' does not depend on any axioms
```

`.claude/rules/protected-surfaces.md` records this as not yet reached. The blocking property is a kernel replay of the built environment. The open question was whether a replay fits CGV CI's time budget.

## Proposed outcome
- CGV CI replays the built environment through the kernel and fails on any declaration that does not re-check. It uses `leanchecker`, which the pinned toolchain (`leanprover/lean4:v4.28.0`) ships in its `bin/`, so no dependency is added.
- The replay covers two sets. `leanchecker --fresh ContractGraph.Main` replays every declaration in the import closure of `ContractGraph.Main` into an empty environment: the Lean library, `leansqlite`, `plausible` and every `ContractGraph` module that `Main` imports. Every protected theorem lives in that closure today. `leanchecker ContractGraph` replays the declarations of every module whose name starts with `ContractGraph`, so a module that `Main` does not import is still checked.
- A self-test step in the same workflow compiles a module with the lines above and a second module that imports it, and fails unless both replay modes reject them. A toolchain bump that changes what `leanchecker` catches then fails CI instead of passing silently.
- `.claude/rules/protected-surfaces.md`, `cgv/CLAUDE.md`, `cgv/README.md` and `docs/assurance/DEVELOPMENT-FRAMEWORK.md` say the kernel replay runs, and the rules stop calling it not yet reached. The generator's header already says that its own axiom check does not reach such a declaration, which stays true, so the generator and the manifest do not change.

Measured on this branch on a local macOS arm64 machine: the plain replay takes 3.5 s and the fresh replay 48 to 57 s. The self-test's fresh replay takes 45 s, because its module imports `Lean`. Recent CGV CI runs took 73 to 184 s, so the job grows by about two minutes.

## Affected users and systems
- Anyone who changes `cgv/`. A declaration that the kernel rejects now fails CGV CI.
- `.github/workflows/cgv-ci.yml` gains two steps.
- `.claude/rules/protected-surfaces.md`, `cgv/CLAUDE.md`, `cgv/README.md`, `docs/assurance/DEVELOPMENT-FRAMEWORK.md`, `intent/2026-09-29-deterministic-evidence-spec.md` (SM-6, CI-7 to CI-9), `docs/TASKS.md`, `JOURNAL.md`.
- The tier gate's evidence line for `cgv/**` and the matching row of `docs/assurance/TIER-LAYER-MAP.md` still say "statement manifest and axiom check". They list what CGV CI runs, so they are incomplete, not wrong. Changing the line changes `scripts/ci/tier-gate.mjs` and its tests, so it gets its own row.

## Constraints
- No CI job calls an LLM, and no secret is used.
- The replay uses the toolchain pinned in `cgv/prover/lean-toolchain`. No new package goes into `lakefile.toml`.
- The manifest `cgv/prover/protected-statements.txt` stays byte-identical.

## Open questions
None. The task row and the issue state the outcome, and the measured times answer the issue's open question.

Three limits stay, and the spec flags each:
- `leanchecker` skips unsafe and `partial` constants (`replay'` in the toolchain's `src/lean/LeanChecker/Replay.lean`). A safe declaration that uses one is replayed only after its dependencies, so the kernel sees an unknown constant and the replay fails. That is inferred from the source, not tested here.
- `leanchecker` checks with the same kernel that built the files. Its own docstring says it is "not an external verifier, simply a tool to detect environment hacking". A second, independently written checker stays not yet reached, as TB-1 already says.
- `implemented_by` and `extern` let compiled code differ from the checked definition. The replay does not see that. TB-1.2 (#48) covers it.
