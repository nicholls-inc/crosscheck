> **Action needed: Resolve markers and checklist before merging**
> You are being asked to verify this governance-note block as PR reviewer because the edit touches a protected surface. Approving means the protected-surface edit merges as governed and traceable; declining means the author must resolve every `REQUIRES HUMAN VERIFICATION:` marker and Review Checklist item first. Full explanation: https://github.com/nicholls-inc/crosscheck/blob/main/docs/gates/protected-surface-amendment.md.

## Protected-Surface Amendment

**Target file(s):** `scripts/ci/tier-gate.mjs` (+ 10 others, see Diff Plan)
**Class:** A (CI enforcement, harness rules, and the governance documents the gates read). One file is a CGV proof surface: `cgv/prover/protected-statements.txt`.
**Matched rule:** `scripts/ci/**`, `.github/workflows/**`, `.claude/rules/**`, `docs/assurance/**`. After this change, `cgv/prover/protected-statements.txt` is also listed.
**Date:** 2026-09-29

### Change Description

**The tier gate** (`scripts/ci/tier-gate.mjs`) changes in five ways:
- It no longer requires or reads an `intent-check` attestation.
- A root `plan.md`, a root `spec.md`, or a governance note counts only if the pull request changes it. A file cited by a `Plan:` or `Spec:` line still counts.
- It requires a `## Protected-surface change` section when the pull request changes a CGV proof surface.
- It prints an evidence report for each class of changed file.
- It prints the fact that the maintainer's merge is the human sign-off.

**CI** changes in four ways:
- The tier gate gains `node:test` tests, and `tier-gate.yml` runs them.
- A new `cgv-ci.yml` runs `cargo test`, `lake build`, the fixture checks, and a theorem-statement manifest diff.
- `spec-audit.yml` is removed, because it ran an LLM.
- `protected-surface-check.yml` is removed, because it ran `/intent-check` through an LLM and never checked governance notes.

**The rules** add the manifest `cgv/prover/protected-statements.txt` to the machine-readable protected list.

**`TIER-LAYER-MAP.md`, `DEVELOPMENT-FRAMEWORK.md`, and PB-1 in `ROADMAP.md`** are updated to match. Each change is traced to a requirement ID in `intent/2026-09-29-deterministic-evidence-spec.md`.

### Rationale

The rationale rests on three concrete triggers:
1. **The vision.** `docs/VISION.md` says no guarantee rests on the judgement of an LLM, yet Tier 3 required an LLM verdict.
2. **The gate is defective.**
   - `tier-gate.mjs` accepted any attestation in the tree. The committed attestation from #134, dated 2026-04-29, satisfied every Tier 3 pull request, and the root `plan.md` from #246 did the same.
   - The gate never read the attestation's verdict or hash.
3. **The maintainer's decisions on 2026-09-29,** recorded in `intent/2026-09-29-crosscheck-monorepo.md`:
   - CGV adopts the full framework.
   - Implement the Tier 3 proposal.
   - No LLM runs in CI, because there is no budget.
   - There is no branch protection, because the repository is private and has no GitHub Pro.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (immediate horizon, item PB-1)
- **Title:** Adopt the AI-native SDLC playbook as the development framework and make every human gate self-explanatory
- **Scope coverage:** PB-1 owns the tier map, the protected-surface hook, and the enforcing CI workflows. This change corrects those deliverables and extends the framework to CGV, which joined the repository under MR-1.

### Authority

- **Authoriser:** harry-nicholls. He directed this change on 2026-09-29.
- **Role:** Maintainer, and the author of the pull request

### Diff Plan

| # | File | Lines | Stage / requirement | Action |
|---|------|-------|---------------------|--------|
| 1 | `scripts/ci/tier-gate.mjs` | whole file | TG-1 to TG-10 | replaced (rules rewritten, `evaluate` exported) |
| 2 | `scripts/ci/tier-gate.test.mjs` | new | TG-1 to TG-10 | added |
| 3 | `.github/workflows/tier-gate.yml` | test step | TG tests | added |
| 4 | `.github/workflows/cgv-ci.yml` | new | CI-1 to CI-6 | added |
| 5 | `.github/workflows/spec-audit.yml` | whole file | WR-1 | removed |
| 6 | `.github/workflows/protected-surface-check.yml` | whole file | WR-2 | removed |
| 7 | `.claude/rules/protected-surfaces.md` | CGV section, amendment pattern, path list | DOC-1, DOC-3 | added (manifest path, manifest text, sign-off without branch protection) |
| 8 | `docs/assurance/TIER-LAYER-MAP.md` | Tier 3 section, new evidence section | DOC-1 | replaced (Tier 3 artefacts), added (CGV evidence, evidence classes) |
| 9 | `docs/assurance/DEVELOPMENT-FRAMEWORK.md` | stages 4 and 5 | DOC-1 | replaced (workflow list, intent-check advisory), added (CGV) |
| 10 | `docs/assurance/ROADMAP.md` | PB-1 scope | DOC-5 | reworded (workflow list, CGV coverage) |
| 11 | `cgv/prover/protected-statements.txt` | new | SM-5 | added |

### Test / Coverage Impact

- **Tier gate.** Until now the gate had no tests. `scripts/ci/tier-gate.test.mjs` covers each TG requirement, and `tier-gate.yml` runs it on every pull request.
- **Statement manifest.** The manifest is new. Its sensitivity was checked locally and is recorded in `plan.md`:
  - two runs give identical output;
  - a proof-only edit leaves the manifest unchanged;
  - a weakened statement changes it;
  - a changed definition that the protected definitions reach changes it.
- **No Class B invariant** doc or property test changes.
- **Attestations.** No attestation or intent-check baseline refresh is needed, because this change removes the attestation from the gate. The committed attestation from #134 stays as a historical record and no longer counts.
- **No silent-regression risk:** there are no BLOCKING lines. The gate becomes stricter about artefact freshness and looser in one respect, the attestation. The rationale argues for that looser direction: the attestation was LLM judgement and was already satisfied by a stale file.

### Review Checklist

- [ ] Rationale is anchored to a concrete trigger (the vision rule, #134 and #246 stale artefacts, the maintainer decisions of 2026-09-29).
- [ ] Authoriser is a named human.
- [ ] PB-1 covers this change.
- [ ] Diff plan enumerates every affected protected file.
- [ ] The tier gate tests cover each TG requirement.
- [ ] Dropping the attestation is argued, not assumed (Rationale, Test / Coverage Impact).
- [ ] This amendment block appears in the PR body.
- [ ] No `REQUIRES HUMAN VERIFICATION:` markers remain.
