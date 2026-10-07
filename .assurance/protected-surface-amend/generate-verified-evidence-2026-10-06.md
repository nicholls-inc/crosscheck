## Protected-Surface Amendment

**Target file(s):** `crosscheck/skills/generate-verified/SKILL.md`
**Class:** A (skill behaviour definition)
**Matched rule:** `crosscheck/skills/*/SKILL.md`
**Date:** 2026-10-06

### Change Description

1. `crosscheck/skills/generate-verified/SKILL.md`: a new Step 7 commits the verified program to a path the caller names with `evidence:`, and calls the MCP tool `dafny_evidence` to emit an evidence record to `.crosscheck/work/dafny/<spec-id>/evidence.json`. It runs only on that opt-in, only after `dafny_verify` accepted the program, and only on a clean work tree. It writes `.crosscheck/.gitignore` with `*` when git does not already ignore `.crosscheck/`. Step 6's Evidence Summary reports the record, or why there is none, and always asks a person to check the drafted statement against the contracts. The front matter and "Arguments" document `evidence:` and `requirement:`.

### Rationale

Task ER-1.6, issue #80. ER-1's acceptance asks for one Crosscheck pipeline to emit an evidence record. ER-1.3 added `dafny_evidence`, and no skill calls it. The spec settles the three questions issue #80 names: when the program is committed, where the record goes, and who drafts the statement and theorems. Intent: `intent/2026-10-06-generate-verified-evidence.md`. Spec: `intent/2026-10-06-generate-verified-evidence-spec.md`. Plan: `intent/2026-10-06-generate-verified-evidence-plan.md`.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (immediate horizon, item ER-1)
- **Title:** Define the evidence record, and emit it from both tools
- **Scope coverage:** ER-1's acceptance names "one Crosscheck pipeline" emitting a record. Task ER-1.6 in `docs/TASKS.md` names `/generate-verified`, a `/crosscheck:protected-surface-amend` note citing ER-1, and a `feat(crosscheck)` commit.

### Authority

- **Authoriser:** harry-nicholls. The merge is the approval.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `crosscheck/skills/generate-verified/SKILL.md` | front matter `argument-hint` | names `evidence:` and `requirement:` |
| 2 | `crosscheck/skills/generate-verified/SKILL.md` | Step 6 Evidence Summary | record lines and the statement review item |
| 3 | `crosscheck/skills/generate-verified/SKILL.md` | new Step 7 | added |
| 4 | `crosscheck/skills/generate-verified/SKILL.md` | Arguments | documents the two arguments |

### Test / Coverage Impact

- A `SKILL.md` has no unit tests. Step 7 was followed by hand on a scratch repository against the real Dafny image, through the MCP server built from this branch. The record passed `scripts/check-evidence-record.mjs`, and its rerun command exited 0.
- A run without `evidence:` behaves as before, except for one line in the Evidence Summary.
- No invariant, eval or `dafny_evidence` rule changes.

### Review Checklist

- [x] Rationale is anchored to ER-1's acceptance and issue #80.
- [x] Authoriser is a named human.
- [x] ER-1 covers wiring a Crosscheck pipeline to emit a record.
- [x] The diff plan names every changed protected file.
- [x] No check is weakened: the step adds a commit and a tool call after the existing verification, and changes nothing before it.
- [ ] REQUIRES HUMAN VERIFICATION: The maintainer accepts that a record is emitted only when the caller names `evidence:`, so a default run emits none.
- [ ] REQUIRES HUMAN VERIFICATION: The maintainer accepts that the skill writes `.crosscheck/.gitignore` with `*` in the caller's repository when `.crosscheck/` is not already ignored.
