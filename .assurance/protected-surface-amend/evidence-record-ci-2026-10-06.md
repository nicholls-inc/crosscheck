> **Action needed: Resolve markers and checklist before merging**
> You are being asked to verify this governance-note block as PR reviewer because the edit touches a protected surface. Approving means the protected-surface edit merges as governed and traceable; declining means the author must resolve every `REQUIRES HUMAN VERIFICATION:` marker and Review Checklist item first. Full explanation: https://github.com/nicholls-inc/crosscheck/blob/main/docs/gates/protected-surface-amendment.md.

## Protected-Surface Amendment

**Target file(s):** `.github/workflows/evidence-record.yml`, `scripts/ci/evidence-record-workflow.test.mjs`, `scripts/ci/tier-gate.mjs`, `scripts/ci/tier-gate.test.mjs`, `docs/assurance/TIER-LAYER-MAP.md`
**Class:** A (CI enforcement, governance documents the gates read)
**Matched rule:** `.github/workflows/**`, `scripts/ci/**`, `docs/assurance/**`
**Date:** 2026-10-06

### Change Description

1. `.github/workflows/evidence-record.yml`: added. The `Evidence Record` workflow runs `node --test scripts/check-evidence-record.test.mjs` on every pull request (EC-1).
2. `scripts/ci/evidence-record-workflow.test.mjs`: added. It runs the workflow step's `run:` script against the committed checker, which must pass, and against a checker whose `checkRecord` returns no problems, which must fail (EC-3).
3. `scripts/ci/tier-gate.mjs`: `EVIDENCE_CLASSES` gains row 5a. `scripts/check-evidence-record.mjs` and `scripts/check-evidence-record.test.mjs` report as checked by the Evidence Record workflow instead of "not yet reached" (EC-2).
4. `scripts/ci/tier-gate.test.mjs`: one case added and the "every class" case extended for row 5a (EC-3).
5. `docs/assurance/TIER-LAYER-MAP.md`: the evidence table gains row 5a (EC-2).

### Rationale

Task ER-1.5. The evidence record checker (ER-1.4, #83) is what ER-1's acceptance asks for, and no workflow ran its 125 tests: the Tier Gate runs only `scripts/ci/*.test.mjs`. A pull request that broke the checker merged with every check green, and the tier gate reported the checker as "not yet reached". Intent: `intent/2026-10-06-evidence-record-ci.md`. Spec: `intent/2026-10-06-evidence-record-ci-spec.md`. Plan: `intent/2026-10-06-evidence-record-ci-plan.md`.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (immediate horizon, item ER-1)
- **Title:** Define the evidence record, and emit it from both tools
- **Scope coverage:** ER-1's acceptance asks for a deterministic checker that rejects a malformed record. Task ER-1.5 in `docs/TASKS.md` names this edit and its protected surface.

### Authority

- **Authoriser:** harry-nicholls. The merge is the approval.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `.github/workflows/evidence-record.yml` | whole file | added |
| 2 | `scripts/ci/evidence-record-workflow.test.mjs` | whole file | added |
| 3 | `scripts/ci/tier-gate.mjs` | `EVIDENCE_CLASSES` | row added |
| 4 | `scripts/ci/tier-gate.test.mjs` | TG-8 cases | case added, case extended |
| 5 | `docs/assurance/TIER-LAYER-MAP.md` | Evidence and sign-off table | row added |

### Test / Coverage Impact

- `node --test scripts/ci/*.test.mjs`, run by the Tier Gate job, gains the EC-3 cases. They fail before the workflow and the row exist.
- The checker's own 125 tests now run on every pull request, in the new Evidence Record check.
- The tier gate's result does not change. Only its information-only evidence report changes, for the two checker files.
- No invariant or eval changes. No intent-check baseline exists for these files.

### Review Checklist

- [ ] Rationale is anchored to a concrete trigger (task ER-1.5, checker PR #83).
- [ ] Authoriser is a named human.
- [ ] ER-1 covers running the checker in CI.
- [ ] The diff plan names every changed protected file.
- [ ] No check is weakened: the Tier Gate runs the same tests, and one more check runs beside it.
- [ ] REQUIRES HUMAN VERIFICATION: the maintainer accepts a separate `Evidence Record` workflow, rather than a step in the Tier Gate job, so a checker failure does not read as a tier gate failure.
