> **Action needed: Resolve markers and checklist before merging**
> You are being asked to verify this governance-note block as PR reviewer because the edit touches a protected surface. Approving means the protected-surface edit merges as governed and traceable; declining means the author must resolve every `REQUIRES HUMAN VERIFICATION:` marker and Review Checklist item first. Full explanation: https://github.com/nicholls-inc/crosscheck/blob/main/docs/gates/protected-surface-amendment.md.

## Protected-Surface Amendment

**Target file(s):** `.github/workflows/cgv-ci.yml`, `.claude/rules/protected-surfaces.md`, `docs/assurance/DEVELOPMENT-FRAMEWORK.md`
**Class:** A (CI enforcement, harness rules, governance documents)
**Matched rule:** `.github/workflows/**`, `.claude/rules/**`, `docs/assurance/**`
**Date:** 2026-10-07

### Change Description

1. `.github/workflows/cgv-ci.yml`: the header cites CI-1 to CI-8, and two steps are added after the manifest step. A self-test (CI-8) compiles, outside the Lake package, a module that adds `theorem bad : False := True.intro` under `debug.skipKernelTC` and a module that imports it, and fails unless `leanchecker` rejects the first and `leanchecker --fresh` rejects the second. The replay step (CI-7) runs `lake env leanchecker --fresh ContractGraph.Main` and `lake env leanchecker ContractGraph` in `cgv/prover`.
2. `.claude/rules/protected-surfaces.md`: in the CGV section, the paragraph that calls a declaration the kernel never checked "not yet reached" is replaced with a description of the replay, and the limits that remain.
3. `docs/assurance/DEVELOPMENT-FRAMEWORK.md`: stage 4's CGV bullet names the kernel replay.

No CGV proof surface changes. `BehaviorModel.lean`, `protected-statements.txt` and `ProtectedStatements.lean` are untouched.

### Rationale

Task TB-1.1, issue #47. The generator's axiom check (SM-6) uses `collectAxioms`, which does not re-check a proof. A theorem added under `set_option debug.skipKernelTC true` with `addDecl` reports no axioms. Reproduced on this branch with Lean 4.28.0: with the issue's lines in `ContractGraph/Types.lean`, `lake build` passes, the generator exits 0 and the manifest is byte-identical, while both `leanchecker` commands exit 1 with `(kernel) declaration type mismatch, 'bad'`. Intent: `intent/2026-10-07-kernel-replay.md`. Spec: `intent/2026-09-29-deterministic-evidence-spec.md` (CI-7 to CI-9). Plan: `intent/2026-10-07-kernel-replay-plan.md`.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (medium-term horizon, item TB-1)
- **Title:** Shrink the trusted base, and measure what remains
- **Scope coverage:** TB-1's scope says "a declaration can skip the Lean kernel" and lists issue #47. Task TB-1.1 in `docs/TASKS.md` is this change.

### Authority

- **Authoriser:** harry-nicholls (maintainer). The merge is the approval.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `.github/workflows/cgv-ci.yml` | new steps after the manifest step; the replay step is named for the built environment and points at CI-9 for its limits | added |
| 2 | `.claude/rules/protected-surfaces.md` | CGV proof surfaces, the paragraph after "A proof must still be a proof": the replay, and the limits that remain (CI-9) | replaced |
| 3 | `docs/assurance/DEVELOPMENT-FRAMEWORK.md` | stage 4, CGV bullet | reworded to name the replay and point at CI-9's four limits |

### Test / Coverage Impact

- The self-test step is the test of the replay. Mutation runs (each `leanchecker` call replaced by `true`, and `debug.skipKernelTC` removed from the bad module) each make it fail. The PR body records the runs.
- CGV CI grows by about 100 s: the fresh replay of `ContractGraph.Main` took 48 to 57 s locally and the self-test's fresh replay 45 s.
- No invariant, eval or attestation changes. The machine-readable path list is unchanged.

### Review Checklist

- [ ] Rationale is anchored to a concrete trigger (issue #47, reproduced on this branch).
- [ ] Authoriser is a named human (not a bot, not an agent).
- [ ] Governing roadmap item TB-1 exists and covers this change.
- [ ] Diff plan enumerates every affected file and section.
- [ ] No invariant is being weakened.
- [ ] This amendment block appears in the PR body.
- [ ] REQUIRES HUMAN VERIFICATION: the extra CGV CI time is acceptable. It was about 100 s on macOS arm64, and a GitHub x86 runner may be slower; this PR's own CGV CI run shows the real figure.
