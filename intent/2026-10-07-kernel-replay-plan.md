# Plan: CGV CI replays every declaration through the Lean kernel

Intent: `intent/2026-10-07-kernel-replay.md`
Spec: `intent/2026-09-29-deterministic-evidence-spec.md` (SM-6 revised, CI-7 to CI-9 added)
Governing roadmap item: TB-1. Task: TB-1.1. Issue: #47. Tier: 3.

This plan is not the root `plan.md`, which belongs to an earlier change. The pull request body cites this file with a `Plan:` line.

## Order of work

1. Write the governance note `.assurance/protected-surface-amend/kernel-replay-2026-10-07.md`, naming the three protected files below.
2. Reproduce the gap. In `cgv/prover`, add the issue's lines to `ContractGraph/Types.lean` with `import Lean`, then run `lake build ContractGraph ContractGraph.Main`, the generator, and both `leanchecker` commands of CI-7. Record that the build, the generator and the manifest diff pass, and that both replays fail. Restore the file with `git checkout`.
3. In `.github/workflows/cgv-ci.yml`, add the CI-8 self-test step and the CI-7 replay step after the manifest step. Run each step's script locally from `cgv/prover` and see the self-test pass and the replay pass on a clean tree.
4. Mutation-check the self-test: replace each `leanchecker` call in it with `true`, and remove `debug.skipKernelTC` from the bad module. Each mutant must make the step fail. Run the replay step on the tree from step 2 and see it fail.
5. Run `actionlint .github/workflows/cgv-ci.yml` if available.
6. Update `.claude/rules/protected-surfaces.md` (CGV section), `docs/assurance/DEVELOPMENT-FRAMEWORK.md` (stage 4, CGV bullet), `cgv/CLAUDE.md` (local command, CI list) and `cgv/README.md` (trust model row).
7. Set TB-1.1 to `done` in `docs/TASKS.md` with this intent as its record, and add TB-1.5 for the tier gate's `cgv/**` evidence line. Add a root `JOURNAL.md` entry.

## Files

| File | Protected | Change |
|---|---|---|
| `.github/workflows/cgv-ci.yml` | yes (`.github/workflows/**`) | self-test step (CI-8) and replay step (CI-7) |
| `.claude/rules/protected-surfaces.md` | yes (`.claude/rules/**`) | the kernel replay replaces the not-yet-reached paragraph |
| `docs/assurance/DEVELOPMENT-FRAMEWORK.md` | yes (`docs/assurance/**`) | stage 4 names the replay |
| `cgv/CLAUDE.md`, `cgv/README.md` | no | local command, CI list, trust model |
| `intent/2026-09-29-deterministic-evidence-spec.md` | no | SM-6, CI-7 to CI-9 |
| `docs/TASKS.md`, `JOURNAL.md` | no | TB-1.1 `done`, row TB-1.5, entry |
| `.assurance/protected-surface-amend/kernel-replay-2026-10-07.md` | no | new governance note |
| `intent/2026-10-07-kernel-replay*.md` | no | stage artefacts |

`cgv/prover/scripts/ProtectedStatements.lean` and `cgv/prover/protected-statements.txt` do not change.

## Risks

- **CI time.** The two fresh replays add about 100 s (measured on macOS arm64; a GitHub x86 runner may be slower) to a job that took 73 to 184 s. Both are single-threaded.
- **The elan proxy.** The self-test calls the toolchain's binaries through `$(lean --print-prefix)/bin`, not through elan proxies, so an elan release without a `leanchecker` proxy does not break it. `lake env leanchecker` resolves the same way.
- **A toolchain bump** may change `leanchecker`'s flags or output. The self-test then fails, which is the intended signal.
- **A false failure from the Lean library.** The fresh replay re-checks the whole Lean library. On Lean 4.28.0 it passes (measured locally). A later toolchain whose library fails replay would block CGV CI until the bump is reconsidered.

## Proof that it worked

- Step 2 shows the gap and that both replay commands close it.
- Step 4 shows the self-test fails under each mutant.
- This pull request's own CGV CI run passes both new steps on the real tree.
