## Protected-Surface Amendment

**Target file(s):** every protected file that the Crosscheck import adds to this repository (52 files, listed in the Diff Plan)
**Class:** A (harness, workflow, governance, rules, evals, CI) and B (invariant specifications)
**Matched rule:** every glob in the machine-readable path list of `.claude/rules/protected-surfaces.md`
**Date:** 2026-09-29

### Change Description

The Crosscheck plugin and the development framework that governs it move from `nicholls-inc/claude-code-marketplace` into this repository, with their git history, by `git filter-repo` and a merge. Every file listed below arrives byte-identical to its state on the marketplace `main` branch, except `.claude/rules/protected-surfaces.md`.

That file gains two changes:

1. A new section, "CGV proof surfaces", which carries CGV's existing protected-surface rules over from this repository's previous rules file.
2. One new line in the machine-readable path list, `cgv/prover/ContractGraph/BehaviorModel.lean`, so the hook and the tier gate guard CGV's trusted semantics.

No skill, agent, invariant, workflow step, threshold, or kill criterion changes behaviour.

### Rationale

Crosscheck and CGV now share one vision (`docs/VISION.md`). One repository lets a change that spans both tools land in one pull request under one set of gates. See `intent/2026-09-29-crosscheck-monorepo.md`.

### Governing Roadmap Item

- REQUIRES HUMAN VERIFICATION: No item in `docs/assurance/ROADMAP.md` covers this move. The reviewer must add one, or record here why the move needs none.

### Authority

- **Authoriser:** REQUIRES HUMAN VERIFICATION: Authoriser draft is unverified. The branch is agent-authored. The expected authoriser is harry-nicholls as the maintainer.
- **Role:** Maintainer

### Test and coverage impact

- The files are unchanged, so the invariant-to-test coverage is unchanged. `crosscheck/mcp-server` tests run in `ci.yml` on this pull request.
- `tier-gate.mjs` now also guards `cgv/prover/ContractGraph/BehaviorModel.lean`.

### Review checklist

- [ ] REQUIRES HUMAN VERIFICATION: The Governing Roadmap Item above is resolved.
- [ ] REQUIRES HUMAN VERIFICATION: The Authoriser above is confirmed.
- [ ] `git diff` against the marketplace `main` shows no content change for any file below except `.claude/rules/protected-surfaces.md`.
- [ ] The two changes to `.claude/rules/protected-surfaces.md` match the Change Description.

### Diff Plan

| # | File | Action |
|---|------|--------|
| 1 | `.claude/rules/protected-surfaces.md` | imported, then amended (CGV section and one glob line) |
| 2 | `.github/workflows/ci.yml` | imported unchanged |
| 3 | `.github/workflows/incident-eval-check.yml` | imported unchanged |
| 4 | `.github/workflows/protected-surface-check.yml` | imported unchanged |
| 5 | `.github/workflows/release.yml` | imported unchanged |
| 6 | `.github/workflows/semantic-pr.yml` | imported unchanged |
| 7 | `.github/workflows/spec-audit.yml` | imported unchanged |
| 8 | `.github/workflows/tier-gate.yml` | imported unchanged |
| 9 | `crosscheck/agents/add-orchestrator.md` | imported unchanged |
| 10 | `crosscheck/agents/auditor.md` | imported unchanged |
| 11 | `crosscheck/agents/byfuglien.md` | imported unchanged |
| 12 | `crosscheck/agents/hellebuyck.md` | imported unchanged |
| 13 | `crosscheck/agents/lowry.md` | imported unchanged |
| 14 | `crosscheck/docs/invariants/extractDifficultyMetrics.md` | imported unchanged |
| 15 | `crosscheck/docs/invariants/parseDafnyOutput.md` | imported unchanged |
| 16 | `crosscheck/docs/invariants/shouldExclude.md` | imported unchanged |
| 17 | `crosscheck/skills/acceptance-oracle-draft/SKILL.md` | imported unchanged |
| 18 | `crosscheck/skills/assurance-init/SKILL.md` | imported unchanged |
| 19 | `crosscheck/skills/assurance-layer-audit/SKILL.md` | imported unchanged |
| 20 | `crosscheck/skills/assurance-probe/SKILL.md` | imported unchanged |
| 21 | `crosscheck/skills/assurance-roadmap-check/SKILL.md` | imported unchanged |
| 22 | `crosscheck/skills/assurance-status/SKILL.md` | imported unchanged |
| 23 | `crosscheck/skills/audit-invariant-consistency/SKILL.md` | imported unchanged |
| 24 | `crosscheck/skills/audit-spec-coverage/SKILL.md` | imported unchanged |
| 25 | `crosscheck/skills/check-regressions/SKILL.md` | imported unchanged |
| 26 | `crosscheck/skills/compare-patches/SKILL.md` | imported unchanged |
| 27 | `crosscheck/skills/correspondence-review/SKILL.md` | imported unchanged |
| 28 | `crosscheck/skills/draft-invariants/SKILL.md` | imported unchanged |
| 29 | `crosscheck/skills/drt-oracle/SKILL.md` | imported unchanged |
| 30 | `crosscheck/skills/extract-code/SKILL.md` | imported unchanged |
| 31 | `crosscheck/skills/generate-verified/SKILL.md` | imported unchanged |
| 32 | `crosscheck/skills/informal-spec/SKILL.md` | imported unchanged |
| 33 | `crosscheck/skills/intent-check/SKILL.md` | imported unchanged |
| 34 | `crosscheck/skills/invariant-coverage-scaffold/SKILL.md` | imported unchanged |
| 35 | `crosscheck/skills/journal-context/SKILL.md` | imported unchanged |
| 36 | `crosscheck/skills/lean-impl/SKILL.md` | imported unchanged |
| 37 | `crosscheck/skills/lean-spec/SKILL.md` | imported unchanged |
| 38 | `crosscheck/skills/lightweight-verify/SKILL.md` | imported unchanged |
| 39 | `crosscheck/skills/locate-fault/SKILL.md` | imported unchanged |
| 40 | `crosscheck/skills/protected-surface-amend/SKILL.md` | imported unchanged |
| 41 | `crosscheck/skills/rationale/SKILL.md` | imported unchanged |
| 42 | `crosscheck/skills/reason/SKILL.md` | imported unchanged |
| 43 | `crosscheck/skills/spec-adversary/SKILL.md` | imported unchanged |
| 44 | `crosscheck/skills/spec-iterate/SKILL.md` | imported unchanged |
| 45 | `crosscheck/skills/suggest-specs/SKILL.md` | imported unchanged |
| 46 | `crosscheck/skills/trace-execution/SKILL.md` | imported unchanged |
| 47 | `docs/assurance/DEVELOPMENT-FRAMEWORK.md` | imported unchanged |
| 48 | `docs/assurance/ROADMAP.md` | imported unchanged |
| 49 | `docs/assurance/TIER-LAYER-MAP.md` | imported unchanged |
| 50 | `evals/README.md` | imported unchanged |
| 51 | `scripts/ci/incident-eval-check.mjs` | imported unchanged |
| 52 | `scripts/ci/tier-gate.mjs` | imported unchanged |
