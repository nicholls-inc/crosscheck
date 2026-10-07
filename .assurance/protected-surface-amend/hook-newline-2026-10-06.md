## Protected-Surface Amendment

**Target file(s):** `.claude/hooks/protected-surface-guard.mjs`, `scripts/ci/protected-surface-guard.test.mjs`
**Class:** A (harness hook, CI enforcement)
**Matched rule:** `.claude/hooks/**`, `scripts/ci/**`
**Date:** 2026-10-06

### Change Description

1. `.claude/hooks/protected-surface-guard.mjs`: `globToRegExp` compiles each glob with the `s` flag, so `**` matches a newline (PG-9).
2. `scripts/ci/protected-surface-guard.test.mjs`: three PG-9 cases. Edits to `protected/n<newline>l.txt` and `protected/x<newline>y/n<newline>l.txt` with no note are blocked, and a new note that names the first unlocks it.

### Rationale

Task PB-1.14. The hook compiled `**` to `.*` with no flags, and `.` matches no newline, so an edit to `docs/assurance/n<newline>l.md` exited 0 with no governance note. A local run on `main` reproduces it. The tier gate had the same fault and gained the `s` flag in PB-1.10. Intent: `intent/2026-10-06-hook-newline.md`. Spec: `intent/2026-10-06-hook-newline-spec.md`. Plan: `intent/2026-10-06-hook-newline-plan.md`.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (immediate horizon, item PB-1)
- **Title:** Adopt the AI-native SDLC playbook as the development framework and make every human gate self-explanatory
- **Scope coverage:** PB-1's scope names the protected-surface hook. Task PB-1.14 in `docs/TASKS.md` is this fix.

### Authority

- **Authoriser:** harry-nicholls. The merge is the approval.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `.claude/hooks/protected-surface-guard.mjs` | `globToRegExp` | `s` flag added |
| 2 | `scripts/ci/protected-surface-guard.test.mjs` | new PG-9 cases | added |

### Test / Coverage Impact

- `node --test scripts/ci/*.test.mjs` gains three PG-9 cases. All three fail against the old hook.
- The hook's result changes only for a path that holds a newline under a `**` glob, which it now blocks.
- No invariant or eval changes.

### Review Checklist

- [x] Rationale is anchored to a local reproduction.
- [x] Authoriser is a named human.
- [x] PB-1 covers the protected-surface hook and its tests.
- [x] The diff plan names every changed protected file.
- [x] No check is weakened: every path blocked before is still blocked.
- [ ] REQUIRES HUMAN VERIFICATION: the maintainer accepts that the tests feed the hook a payload and do not drive a harness edit of a file whose name holds a newline.
