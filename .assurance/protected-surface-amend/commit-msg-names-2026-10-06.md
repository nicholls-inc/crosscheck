> **Action needed: Resolve markers and checklist before merging**
> You are being asked to verify this governance-note block as PR reviewer because the edit touches a protected surface. Approving means the protected-surface edit merges as governed and traceable; declining means the author must resolve every `REQUIRES HUMAN VERIFICATION:` marker and Review Checklist item first. Full explanation: https://github.com/nicholls-inc/crosscheck/blob/main/docs/gates/protected-surface-amendment.md.

## Protected-Surface Amendment

**Target file(s):** `scripts/ci/commit-msg.test.mjs`
**Class:** A (CI enforcement)
**Matched rule:** `scripts/ci/**`
**Date:** 2026-10-06

### Change Description

`scripts/ci/commit-msg.test.mjs` is added. It runs `.husky/commit-msg` in a scratch repository with `npx` stubbed, and checks that a `docs:` or `refactor:` commit staging a behavioural artefact fails, including names that git quotes or that hold a newline, and that a `fix:` commit or a non-behavioural name passes (CM-5). No existing protected file changes. `.husky/commit-msg`, which the test covers, is not protected.

### Rationale

Task PB-1.13. `.husky/commit-msg` read staged names with `git diff --cached --name-only`, which C-quotes a name with a non-ASCII byte, a double quote, a backslash or a control character. The quoted name ends in `"`, so `crosscheck/skills/é/SKILL.md` escaped the commit-type check under `docs: x`. Reproduced in a scratch repository: the plain name exited 1, and `é`, `a"b` and `n<newline>l` names exited 0. The hook had no test. CI runs only `scripts/ci/*.test.mjs`, so the test lives there, as `scripts/ci/protected-surface-guard.test.mjs` does for another hook. Intent: `intent/2026-10-06-commit-msg-names.md`. Spec: `intent/2026-10-06-commit-msg-names-spec.md`. Plan: `intent/2026-10-06-commit-msg-names-plan.md`.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (immediate horizon, item PB-1)
- **Title:** Adopt the AI-native SDLC playbook as the development framework and make every human gate self-explanatory
- **Scope coverage:** PB-1 covers the deterministic checks of the development framework and the task queue. Task PB-1.13 in `docs/TASKS.md` is this fix.

### Authority

- **Authoriser:** harry-nicholls. The merge is the approval.
- **Role:** Maintainer

### Diff Plan

| # | File | Lines | Invariant ID / Stage | Action |
|---|------|-------|----------------------|--------|
| 1 | `scripts/ci/commit-msg.test.mjs` | whole file | CM-5 | added |

### Test / Coverage Impact

- `node --test scripts/ci/*.test.mjs` gains the CM-5 cases. The cases for the unusual names and for `refactor:` fail against the old hook.
- No existing test, invariant or eval changes. No attestation or intent-check baseline depends on this file.

### Review Checklist

- [x] Rationale is anchored to a local reproduction in a scratch repository.
- [x] Authoriser is a named human.
- [x] PB-1 covers the deterministic checks and the task queue.
- [x] The diff plan names every changed protected file.
- [x] No check is weakened: the file is new, and no protected file is edited.
- [ ] REQUIRES HUMAN VERIFICATION: the maintainer accepts that the test stubs `npx`, so commitlint is not exercised, and that a per-commit type check in CI is not yet reached.
