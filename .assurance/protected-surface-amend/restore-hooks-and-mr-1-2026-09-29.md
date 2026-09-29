## Protected-Surface Amendment

**Target file(s):** `.claude/hooks/protected-surface-guard.mjs`, `.claude/hooks/settings-snippet.json`, `docs/assurance/ROADMAP.md`
**Class:** A (harness hooks, governance documents the gates read)
**Matched rule:** `.claude/hooks/**`, `docs/assurance/**`
**Date:** 2026-09-29

### Change Description

1. The protected-surface hook returns. `.claude/hooks/protected-surface-guard.mjs` and `.claude/hooks/settings-snippet.json` are byte-identical to the marketplace's last commit before nicholls-inc/claude-code-marketplace#251. `.claude/settings.json`, which registers the hook, is restored from the same commit.
2. `docs/assurance/ROADMAP.md` gains item MR-1 (status Done) for the move into this repository.
3. The note that governed #42 now has its two markers resolved. It moves to `.assurance/archive/protected-surface-amend/`, where neither the hook nor the tier gate reads it.

### Rationale

#42 left these files out on purpose, because hooks run automatically in every session and an agent should not install them unasked. The maintainer has now asked for them. MR-1 is the roadmap item that the #42 note lacked. Archiving that note matters because its Diff Plan names all 52 protected files. The hook and the tier gate accept any note that names a file, so leaving it in place would allow edits to all 52 without a new amendment.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (immediate horizon, item MR-1)
- **Title:** Consolidate Crosscheck and the contract graph verifier into one repository under one vision
- **Scope coverage:** MR-1 covers restoring the governance tooling that the move left behind.

### Authority

- **Authoriser:** harry-nicholls. He directed each change in this note on 2026-09-29.
- **Role:** Maintainer

### Test and coverage impact

A PreToolUse payload for `cgv/prover/ContractGraph/BehaviorModel.lean` or `crosscheck/skills/reason/SKILL.md` makes the hook exit 2 (blocked). A payload for `cgv/src/main.rs` makes it exit 0.

### Review checklist

- [ ] The hook files match the marketplace commit before #251.
- [ ] MR-1 matches the move that landed in #18, #42, and nicholls-inc/claude-code-marketplace#251.

### Diff Plan

| # | File | Action |
|---|------|--------|
| 1 | `.claude/hooks/protected-surface-guard.mjs` | restored unchanged |
| 2 | `.claude/hooks/settings-snippet.json` | restored unchanged |
| 3 | `docs/assurance/ROADMAP.md` | added item MR-1 |
