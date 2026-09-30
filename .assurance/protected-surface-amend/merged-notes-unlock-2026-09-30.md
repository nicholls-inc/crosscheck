## Protected-Surface Amendment

**Target file(s):** `.claude/hooks/protected-surface-guard.mjs`, `scripts/ci/protected-surface-guard.test.mjs`, `.claude/rules/protected-surfaces.md`, `docs/assurance/DEVELOPMENT-FRAMEWORK.md`, `crosscheck/skills/assurance-init/SKILL.md`
**Class:** A (harness hooks and rules, CI enforcement, governance documents, skill behaviour)
**Matched rule:** `.claude/hooks/**`, `scripts/ci/**`, `.claude/rules/**`, `docs/assurance/**`, `crosscheck/skills/*/SKILL.md`
**Date:** 2026-09-30

### Change Description

1. `.claude/hooks/protected-surface-guard.mjs`: a governance-note block allows an edit only if its text is not in any note file on the default-branch commit (`origin/HEAD`, else `origin/main`), and only for paths its own note file did not already name there. If neither ref resolves, an edit to a protected file is blocked with a message that names the fix. The gate message's reason clause says "no governance-note block that is new on this branch".
2. `scripts/ci/protected-surface-guard.test.mjs`: new. Scratch-repository tests of the hook, run by the Tier Gate job's `node --test scripts/ci/*.test.mjs`.
3. `.claude/rules/protected-surfaces.md`: the deterministic-layer bullet states the new rule. The path list does not change.
4. `docs/assurance/DEVELOPMENT-FRAMEWORK.md`: stage 3 states the new rule.
5. `crosscheck/skills/assurance-init/SKILL.md`: the step 7.5b hook contract states the new rule.

### Rationale

Notes stay on `main` after their change merges, and the hook counted every note. On `main` at 608ca86 the hook allows edits to 30 of the 58 tracked protected files with no new note. The Tier Gate already refuses notes from earlier changes, and the hook now applies the same rule locally. Intent: `intent/2026-09-30-merged-notes-unlock.md`. Spec: `intent/2026-09-30-merged-notes-unlock-spec.md`. Plan: `intent/2026-09-30-merged-notes-unlock-plan.md`.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (immediate horizon, item PB-1)
- **Title:** Adopt the AI-native SDLC playbook as the development framework and make every human gate self-explanatory
- **Scope coverage:** PB-1's scope names the protected-surface hook. Task PB-1.3 in `docs/TASKS.md` is this fix.

### Authority

- **Authoriser:** harry-nicholls. The merge is the approval.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `.claude/hooks/protected-surface-guard.mjs` | note lookup, gate message | changed |
| 2 | `scripts/ci/protected-surface-guard.test.mjs` | whole file | added |
| 3 | `.claude/rules/protected-surfaces.md` | deterministic layer | reworded |
| 4 | `docs/assurance/DEVELOPMENT-FRAMEWORK.md` | stage 3 | reworded |
| 5 | `crosscheck/skills/assurance-init/SKILL.md` | step 7.5b | reworded |

### Test / Coverage Impact

- `node --test scripts/ci/*.test.mjs` now includes the PG-7 cases.
- No invariant or eval changes.

### Review Checklist

- [x] On a clean checkout of `main` with the new hook, no tracked protected file is allowed.
- [x] The rules-file fail-open and fail-closed behaviour is unchanged.
