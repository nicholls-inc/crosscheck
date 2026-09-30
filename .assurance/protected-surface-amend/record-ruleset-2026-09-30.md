## Protected-Surface Amendment

**Target file(s):** `.claude/rules/protected-surfaces.md`, `docs/assurance/DEVELOPMENT-FRAMEWORK.md`, `docs/assurance/TIER-LAYER-MAP.md`, `scripts/ci/tier-gate.mjs`
**Class:** A (harness rules, governance documents the gates read, CI enforcement)
**Matched rule:** `.claude/rules/**`, `docs/assurance/**`, `scripts/ci/**`
**Date:** 2026-09-30

### Change Description

1. `.claude/rules/protected-surfaces.md`, amendment pattern step 1: the paragraph saying the repository has no branch protection because it is private is replaced by what the `default` ruleset enforces. It notes that no CODEOWNERS file exists, and that the maintainer merges by bypassing the ruleset, which is the approval. Approval by someone other than the author is described as not yet reached, blocked by the absence of a second person with write access.
2. `docs/assurance/DEVELOPMENT-FRAMEWORK.md`: the opening section and stage 5 say that the ruleset requires no status checks, so CI cannot block a merge.
3. `docs/assurance/TIER-LAYER-MAP.md`: the opening section and "Evidence and sign-off" say the same, and add that every merge bypasses the ruleset.
4. `scripts/ci/tier-gate.mjs`: the TG-9 pass line says that the maintainer's merge bypasses the default-branch ruleset and that the ruleset requires no status checks. No check or exit code changes.

### Rationale

The repository became public, and a ruleset on the default branch was created on 2026-09-30. Six live documents still say that the repository is private with no branch protection. Agents and gates read these files. The maintainer asked on 2026-09-30 for the statements to be corrected. Intent: `intent/2026-09-30-record-ruleset.md`. Spec: `intent/2026-09-30-record-ruleset-spec.md`.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (immediate horizon, item PB-1)
- **Title:** Adopt the AI-native SDLC playbook as the development framework and make every human gate self-explanatory
- **Scope coverage:** PB-1 owns the framework documents and the gate explainers, and it recorded the move to "the maintainer's merge is the human sign-off" (`intent/2026-09-29-deterministic-evidence.md`). This change corrects the facts that sign-off statement rests on.

### Authority

- **Authoriser:** harry-nicholls, who directed this change on 2026-09-30. The merge is the approval.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `.claude/rules/protected-surfaces.md` | Amendment pattern, step 1 | replaced: ruleset, bypass merge, not yet reached |
| 2 | `docs/assurance/DEVELOPMENT-FRAMEWORK.md` | opening section; stage 5 | reworded: no required status checks |
| 3 | `docs/assurance/TIER-LAYER-MAP.md` | opening section; Evidence and sign-off | reworded: ruleset, bypass merge, no required status checks |
| 4 | `scripts/ci/tier-gate.mjs` | TG-9 pass line | reworded |

### Test / Coverage Impact

- `scripts/ci/tier-gate.test.mjs` asserts the prefix `Human sign-off: the maintainer's merge`, which is unchanged. No test changes.
- No invariant, property test, or gate behaviour changes.

### Review Checklist

- [ ] The ruleset facts match the repository settings.
- [ ] Historical records (earlier intents, specs, plans and notes) are left as written.
- [ ] No gate, check, or exit code changes.
