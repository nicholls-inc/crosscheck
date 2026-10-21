# Plan: Name CGV as Layer 3 reach for Python in `/assurance-layer-audit`

Intent: `intent/2026-10-21-cgv-layer-3-python.md`. Spec: `intent/2026-10-21-cgv-layer-3-python-spec.md`.

## Files that change

| File | Lines today | Change |
|---|---|---|
| `crosscheck/skills/assurance-layer-audit/SKILL.md` | Instructions paragraph (line 24); Step 4 Layer 3 Python / TypeScript bullet (line 115); Verification Checklist Layer 1 line (line 302) | CL-1 to CL-7 |
| `.assurance/protected-surface-amend/cgv-layer-3-python-2026-10-21.md` | new | governance note naming the file |
| `docs/TASKS.md` | VA-1.19 row | `done`, record |
| `crosscheck/JOURNAL.md` | new entry | |

## Order of work

1. Commit the intent, spec and plan.
2. Write the governance note, so the PreToolUse hook allows the edit.
3. Edit the skill.
4. Update the VA-1.19 row and `crosscheck/JOURNAL.md`.
5. Run the checks below.

## Risks

- Overclaiming CGV's reach. The wording is taken from the Layer 3 row of `crosscheck/docs/assurance-hierarchy.md` and from `cgv/README.md`, and the exit-0 qualifiers travel with it.
- VA-1.8 edits nearby lines (the Go bullet, the "honest reading" line, the example table). Whichever merges second resolves a text conflict.
- ER-1.20 adds a `cgv_check` tool. When it lands, the bullet's `contracts check` pointer can name it; ER-1.21 owns the routing.

## Proof

- CL-5: `grep -n "not formal" crosscheck/skills/assurance-layer-audit/SKILL.md` prints nothing.
- CL-6: `grep -n "loses most of Layer 1" crosscheck/skills/assurance-layer-audit/SKILL.md` prints nothing.
- CL-8: `go run ./crosscheck/conformance crosscheck` passes, and `git diff --stat origin/main` touches only the files above.
- `node --test scripts/ci/*.test.mjs`, `node scripts/ci/skill-references.mjs`, `node scripts/ci/task-queue.mjs check` and `node scripts/ci/tier-gate.mjs` with the PR body pass.
