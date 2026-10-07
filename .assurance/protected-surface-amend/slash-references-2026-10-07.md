> **Action needed: Resolve markers and checklist before merging**
> You are being asked to verify this governance-note block as PR reviewer because the edit touches a protected surface. Approving means the protected-surface edit merges as governed and traceable; declining means the author must resolve every `REQUIRES HUMAN VERIFICATION:` marker and Review Checklist item first. Full explanation: https://github.com/nicholls-inc/crosscheck/blob/main/docs/gates/protected-surface-amendment.md.

## Protected-Surface Amendment

**Target file(s):** `scripts/ci/skill-references.mjs` (+ others, see Diff Plan)
**Class:** A (CI enforcement, governance documents, skill behaviour definitions)
**Matched rule:** `scripts/ci/**`, `.github/workflows/**`, `docs/assurance/**`, `crosscheck/skills/*/SKILL.md`
**Date:** 2026-10-07

### Change Description

1. `scripts/ci/skill-references.mjs` (new): a checker that reads the git index, finds each `/name` and `/<plugin>:<name>` in `crosscheck/**/*.md` (except archives, snapshots, research notes and gh-aw examples), resolves it against `crosscheck/skills/`, `crosscheck/agents/` and the allowlist `crosscheck/slash-allowlist.txt`, and fails on a reference that does not resolve or on a `crosscheck/docs/skills.md` that differs from the generated catalogue. `--write` regenerates the catalogue.
2. `scripts/ci/skill-references.test.mjs` (new) and the fixture under `scripts/ci/fixtures/skill-references/` (new): `scripts/ci/fixtures/skill-references/crosscheck/skills/alpha/SKILL.md`, `scripts/ci/fixtures/skill-references/crosscheck/agents/beta.md`, `scripts/ci/fixtures/skill-references/crosscheck/docs/broken.md`, `scripts/ci/fixtures/skill-references/crosscheck/docs/skills.md`, `scripts/ci/fixtures/skill-references/crosscheck/slash-allowlist.txt`.
3. `scripts/ci/pre-commit.mjs`: a third check, which runs the checker when a commit stages a Markdown file under `crosscheck/` or the allowlist. `scripts/ci/pre-commit.test.mjs` copies the new script and adds a case.
4. `.github/workflows/skill-references.yml` (new): runs the checker on every pull request and every push to `main`.
5. `docs/assurance/DEVELOPMENT-FRAMEWORK.md`: stage 5 lists the new workflow.
6. `crosscheck/skills/journal-context/SKILL.md`: line 30 names the unbuilt `journal-lint` skill without a slash, so the text no longer reads as an instruction to run it. No behaviour changes.

### Rationale

Issue #36, decided in `intent/2026-10-07-backlog-review-decisions.md` ("#36 → PB-1.18"). No check resolved a slash-reference outside five conformance doc files and agent bodies. `crosscheck/skills/journal-context/SKILL.md:30` names `/journal-lint`, which does not exist, and `crosscheck/docs/skills.md` said "all 29 skills" while `crosscheck/skills/` has 30. Intent: `intent/2026-10-07-slash-references.md`. Spec: `intent/2026-10-07-slash-references-spec.md`. Plan: `intent/2026-10-07-slash-references-plan.md`.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (immediate horizon, item PB-1)
- **Title:** Adopt the AI-native SDLC playbook as the development framework and make every human gate self-explanatory
- **Scope coverage:** PB-1 covers the deterministic CI and pre-commit checks of the development framework. Task PB-1.18 in `docs/TASKS.md` is this change, and the decision record files #36 under PB-1.

### Authority

- **Authoriser:** harry-nicholls. The merge is the approval.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `scripts/ci/skill-references.mjs` | whole file | added |
| 2 | `scripts/ci/skill-references.test.mjs` | whole file | added |
| 3 | `scripts/ci/fixtures/skill-references/**` | five fixture files listed above | added |
| 4 | `scripts/ci/pre-commit.mjs` | `CHECKS` | added a check |
| 5 | `scripts/ci/pre-commit.test.mjs` | `COPIED`, new case | added |
| 6 | `.github/workflows/skill-references.yml` | whole file | added |
| 7 | `docs/assurance/DEVELOPMENT-FRAMEWORK.md` | stage 5 workflow list | added a bullet |
| 8 | `crosscheck/skills/journal-context/SKILL.md` | line 30 | reworded (no semantic change) |
| 9 | `crosscheck/skills/journal-context/docs/invariants/journal-context.md` | two mentions of `journal-lint` | slash dropped (wording only; not a protected path, since the hook anchors `crosscheck/docs/invariants/**` at the repository root) |

### Test / Coverage Impact

- `node --test scripts/ci/*.test.mjs`, run by the Tier Gate workflow, gains the SR-10 cases and one pre-commit case.
- The pre-commit hook now fails a commit that stages a `crosscheck/` Markdown file with a reference that does not resolve, or that leaves the catalogue stale.
- One invariant doc changes in wording only (diff plan row 9); no eval changes. No attestation or intent-check baseline exists for these files.
- REQUIRES HUMAN VERIFICATION: the checker skips `crosscheck/docs/add/.retrospective/`, `crosscheck/.assurance/`, `crosscheck/docs/research/`, `crosscheck/docs/reports/` and `crosscheck/docs/examples/workflows/`, where the decision for #36 named all of `crosscheck/**/*.md`. The spec's "Files not checked" concern gives the reason. Confirm this line, or name the files to rewrite instead.

### Review Checklist

- [ ] Rationale is anchored to a concrete trigger (issue #36).
- [ ] Authoriser is a named human (not a bot, not an agent).
- [ ] Governing roadmap item exists and actually covers this change.
- [ ] Diff plan enumerates every affected file.
- [ ] No invariant is being weakened purely to make a failing test pass.
- [ ] This amendment block appears in the PR body.
- [ ] All `REQUIRES HUMAN VERIFICATION:` markers above have been resolved.
