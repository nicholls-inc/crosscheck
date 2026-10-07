> **Action needed: Resolve markers and checklist before merging**
> You are being asked to verify this governance-note block as PR reviewer because the edit touches a protected surface. Approving means the protected-surface edit merges as governed and traceable; declining means the author must resolve every `REQUIRES HUMAN VERIFICATION:` marker and Review Checklist item first. Full explanation: https://github.com/nicholls-inc/crosscheck/blob/main/docs/gates/protected-surface-amendment.md.

## Protected-Surface Amendment

**Target file(s):** `scripts/ci/incident-eval-check.mjs` (+ 2 others, see Diff Plan)
**Class:** A (CI enforcement, governance documents)
**Matched rule:** `scripts/ci/**`, `docs/assurance/**`
**Date:** 2026-10-06

### Change Description

1. `scripts/ci/incident-eval-check.mjs`: `findIncidentId` replaces the unanchored `/Fixes-Incident:\s*(\S+)/i` with the whole-line pattern `^[ \t]*Fixes-Incident:[ \t]*(\S+)[ \t]*$` (IE-9), and splits the PR body into lines before matching it, as commit messages already were (IE-5 revised).
2. `scripts/ci/incident-eval-check.test.mjs`: the case that let a body trigger line with no value take the next line's id is replaced by its opposite, and the IE-9 cases are added, including the body line and the commit line of #62 verbatim (IE-6 revised).
3. `docs/assurance/DEVELOPMENT-FRAMEWORK.md`: stage 5 states the whole-line rule in place of "anywhere in a line".

### Rationale

The run for #62 (Incident Eval Check run 37527444185) failed after the merge and named the incident `` <id>` ``. #62 fixed no incident. Its body quoted the trigger inside a list item, and one of its commits wrapped a sentence so that a line began with the trigger. Task PB-1.16 in `docs/TASKS.md` records this. Intent: `intent/2026-10-06-incident-line.md`. Spec: `intent/2026-10-06-incident-line-spec.md`. Plan: `intent/2026-10-06-incident-line-plan.md`.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (immediate horizon, item PB-1)
- **Title:** Adopt the AI-native SDLC playbook as the development framework and make every human gate self-explanatory
- **Scope coverage:** PB-1's scope names `incident-eval-check.yml` as part of the deterministic CI it adopts. Task PB-1.16 is this fix.

### Authority

- **Authoriser:** harry-nicholls. The merge is the approval. REQUIRES HUMAN VERIFICATION: an agent drafted this note; the maintainer confirms by merging or replaces the name.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `scripts/ci/incident-eval-check.mjs` | `findIncidentId` | replaced |
| 2 | `scripts/ci/incident-eval-check.test.mjs` | IE-5 body case, new IE-9 cases | replaced and added |
| 3 | `docs/assurance/DEVELOPMENT-FRAMEWORK.md` | stage 5, `incident-eval-check.yml` bullet | reworded |

### Test / Coverage Impact

- `node --test scripts/ci/*.test.mjs` gains the IE-9 cases. They fail on `origin/main` and pass with this change.
- The check's result changes in one direction for some inputs: a line that quotes, lists or wraps the trigger no longer applies the check, so those runs exit 0 where they exited 1. This also covers a reference written as a sentence, such as `Fixes-Incident: INC-7 (the outage)`. The `incident` label still forces the check. The spec lists this under "Not yet reached".
- No invariant doc or eval changes. No attestation or intent-check baseline exists for this script.

### Review Checklist

- [ ] Rationale is anchored to a concrete trigger (run 37527444185 for #62, task PB-1.16).
- [ ] Authoriser is a named human (not a bot, not an agent).
- [ ] Governing roadmap item exists and actually covers this change.
- [ ] Diff plan enumerates every affected file and section.
- [ ] The narrower trigger is a deliberate trade, argued in the spec, not a weakening to make a test pass.
- [ ] This amendment block appears in the PR body.
- [ ] All `REQUIRES HUMAN VERIFICATION:` markers above have been resolved.
