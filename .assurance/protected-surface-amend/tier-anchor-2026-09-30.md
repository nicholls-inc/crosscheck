## Protected-Surface Amendment

**Target file(s):** `scripts/ci/tier-gate.mjs`, `scripts/ci/tier-gate.test.mjs`, `docs/assurance/TIER-LAYER-MAP.md`
**Class:** A (CI enforcement, governance documents)
**Matched rule:** `scripts/ci/**`, `docs/assurance/**`
**Date:** 2026-09-30

### Change Description

1. `scripts/ci/tier-gate.mjs`: the `Tier:` declaration counts only at the start of a line, after an optional indent and no list or quote marker (TG-1 revised). The first `Tier:` line is the declaration even when invalid, and an invalid one fails. A `Tier:` line and a `tier:N` label must agree, and so must two labels; disagreement fails. The evidence report's class table is replaced: each row is checked or not yet reached, and a not-yet-reached row names its blocking property and open question. `crosscheck/conformance/**` and the protected-surface hook gain checked rows. Agent and reviewer instructions (`CLAUDE.md`, `AGENTS.md`, `REVIEW.md`) join the prompt-text row, root `docs/invariants/**` and prose each get a not-yet-reached row, and no line says "none required at this tier" (TG-8 revised).
2. `scripts/ci/tier-gate.test.mjs`: the TG-11 cases are added, and the two existing TG-8 cases assert the new line format.
3. `docs/assurance/TIER-LAYER-MAP.md`: the declaration rule and the evidence table state the revised TG-1 and TG-8. The map already said a line and a label "must agree", but the gate never enforced it and let the line win. That requirement is kept, and the gate now enforces it. An earlier revision of this PR dropped the requirement to match the code. The maintainer reversed that on review (thread on `TIER-LAYER-MAP.md`, decision 4b).

### Rationale

Issue #50. The gate matched `Tier:` anywhere in the PR body, so pasted text could declare a tier (https://github.com/nicholls-inc/crosscheck/pull/46#discussion_r4136560939). The pass report labelled unchecked code "none required at this tier", where `docs/VISION.md` requires "not yet reached" with the blocking property and the open question (https://github.com/nicholls-inc/crosscheck/pull/46#discussion_r4136326009). Intent: `intent/2026-09-30-tier-anchor.md`. Spec: `intent/2026-09-30-tier-anchor-spec.md`. Plan: `intent/2026-09-30-tier-anchor-plan.md`.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (immediate horizon, item PB-1)
- **Title:** Adopt the AI-native SDLC playbook as the development framework and make every human gate self-explanatory
- **Scope coverage:** PB-1's scope names `tier-gate.yml` with the gate's own tests, and lists issue #50. Task PB-1.4 in `docs/TASKS.md` is this fix.

### Authority

- **Authoriser:** harry-nicholls. The merge is the approval.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `scripts/ci/tier-gate.mjs` | `parseDeclaredTier`, `EVIDENCE_CLASSES`, `evidenceReport` | changed |
| 2 | `scripts/ci/tier-gate.test.mjs` | TG-1 and TG-8 cases | added and changed |
| 3 | `docs/assurance/TIER-LAYER-MAP.md` | declaration, Evidence and sign-off | reworded |

### Test / Coverage Impact

- `node --test scripts/ci/*.test.mjs` gains the TG-11 cases.
- The gate's pass or fail result changes for a PR body whose only `Tier:` is mid-line or after a list or quote marker, whose first `Tier:` line is not exactly `Tier: 1`, `2` or `3`, or whose `Tier:` line and `tier:N` label disagree. The evidence report is information only.
- No invariant or eval changes.

### Review Checklist

- [x] Rationale is anchored to issue #50.
- [x] Authoriser is a named human.
- [x] PB-1 covers the tier gate and lists #50.
- [x] No line in the report of every tracked file says "none required at this tier".
- [x] The "must agree" rule for a line and a label is kept and enforced, not dropped.
