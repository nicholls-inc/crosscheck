> **Action needed: Resolve markers and checklist before merging**
> You are being asked to verify this governance-note block as PR reviewer because the edit touches a protected surface. Approving means the protected-surface edit merges as governed and traceable; declining means the author must resolve every `REQUIRES HUMAN VERIFICATION:` marker and Review Checklist item first. Full explanation: https://github.com/nicholls-inc/crosscheck/blob/main/docs/gates/protected-surface-amendment.md.

## Protected-Surface Amendment

**Target file(s):** `scripts/ci/tier-gate.mjs`, `scripts/ci/tier-gate.test.mjs`, `docs/assurance/TIER-LAYER-MAP.md`
**Class:** A (CI enforcement, governance documents)
**Matched rule:** `scripts/ci/**`, `docs/assurance/**`
**Date:** 2026-09-30

### Change Description

1. `scripts/ci/tier-gate.mjs`: `citedExisting` reads every `Intent:`, `Spec:` or `Plan:` citation line, not only the first, and the requirement is met when any of them cites a valid path. A path is valid only when it resolves, through symlinks, to a regular file whose real path lies inside the repository. A directory, a path outside the repository, and a symlink that points outside it no longer count (TG-12). The citation line format does not change.
2. `scripts/ci/tier-gate.test.mjs`: the TG-13 cases are added. They cover the `+ ` marker, near misses that are not citations (`-Plan:`, `>> Plan:`, `1. Plan:`, `- [ ] Plan:`), several citation lines in either order, directories, paths outside the repository, and symlinks.
3. `docs/assurance/TIER-LAYER-MAP.md`: the Tier 2 and Tier 3 artefact lists say that a citation names a file in the repository, and that any citation line counts (DOC-7).

### Rationale

Issue #49. Only the first citation line counted, so `Plan: TBD` followed by a real `Plan:` line failed (https://github.com/nicholls-inc/crosscheck/pull/46#discussion_r4136616243). `existsSync` accepted a directory or a `../x` path (https://github.com/nicholls-inc/crosscheck/pull/46#discussion_r4136616250). The citation tests were positive only (https://github.com/nicholls-inc/crosscheck/pull/46#discussion_r4136616253). Intent: `intent/2026-09-30-citation-rule.md`. Spec: `intent/2026-09-30-citation-rule-spec.md`. Plan: `intent/2026-09-30-citation-rule-plan.md`.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (immediate horizon, item PB-1)
- **Title:** Adopt the AI-native SDLC playbook as the development framework and make every human gate self-explanatory
- **Scope coverage:** PB-1's scope names `tier-gate.yml` with the gate's own tests, and lists issue #49. Task PB-1.5 in `docs/TASKS.md` is this fix.

### Authority

- **Authoriser:** harry-nicholls. The merge is the approval.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `scripts/ci/tier-gate.mjs` | `citedExisting` | changed |
| 2 | `scripts/ci/tier-gate.test.mjs` | TG-2, TG-3 and TG-4 citation cases | added |
| 3 | `docs/assurance/TIER-LAYER-MAP.md` | Tier 2 and Tier 3 artefact lists | reworded |

### Test / Coverage Impact

- `node --test scripts/ci/*.test.mjs` gains the TG-13 cases.
- The gate's result changes in two directions. A body whose first citation line is invalid but a later one is valid now passes. A body that cites a directory, a path outside the repository, or a symlink out of it now fails.
- No invariant or eval changes. No attestation or intent-check baseline exists for this gate.

### Review Checklist

- [x] Rationale is anchored to issue #49.
- [x] Authoriser is a named human.
- [x] PB-1 covers the tier gate and lists #49.
- [x] The diff plan names every changed protected file.
- [x] No requirement is weakened: every counting citation still names a regular file in the repository.
- [ ] The maintainer confirms the known gaps in TG-12 (content is not read, and `.git/` files count) are acceptable.
