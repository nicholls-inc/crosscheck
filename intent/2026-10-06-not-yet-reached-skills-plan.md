# Plan: Say "not yet reached" in Crosscheck's skills and agents

Intent: `intent/2026-10-06-not-yet-reached-skills.md`. Spec: `intent/2026-10-06-not-yet-reached-skills-spec.md`.

## Files that change

| File | Lines today | Change |
|---|---|---|
| `crosscheck/agents/hellebuyck.md` | skill table `/spec-adversary` row; Task Classification intro; "Best-effort honesty" gate; "Layer 6 is best-effort" guideline | NR-2 |
| `crosscheck/skills/audit-spec-coverage/SKILL.md` | "best-effort, like all Layer 6 work"; Step 9 intro | NR-2 |
| `crosscheck/skills/audit-invariant-consistency/SKILL.md` | layer note; `/intent-check` comparison row; blind spot 2; closing note of "What this does NOT catch"; Step 9 intro | NR-2, NR-4 |
| `crosscheck/skills/spec-adversary/SKILL.md` | frontmatter description; Step 7 intro | NR-2 |
| `crosscheck/skills/assurance-layer-audit/SKILL.md` | Layer 2 language bullets; Layer 6 reach paragraph; reach table Layer 2 and Layer 6 rows; "Layer 6 work last" heuristic; checklist line | NR-2, NR-3 |
| `crosscheck/skills/drt-oracle/SKILL.md` | Aeneas reference bullet | NR-5 |
| `crosscheck/skills/acceptance-oracle-draft/SKILL.md` | CRUCIAL RULE; Step 7 heading text and example; frontmatter description | NR-6, NR-12 |
| `crosscheck/skills/intent-check/SKILL.md` | Step 9 intro sentence | NR-7 |
| `crosscheck/skills/rationale/SKILL.md` | description; Step 4 `[STATIC]` bullet; Step 5 example checklist and Summary table; Step 6 Evidence Summary and closing sentence | NR-9, NR-10, NR-11 |
| `.assurance/protected-surface-amend/not-yet-reached-skills-2026-10-06.md` | new | governance note naming all nine files |
| `docs/TASKS.md` | VA-1.5 row; new VA-1.8 row | done, record; Layer 5 strength and Layer 3 "aspirational" |
| `crosscheck/JOURNAL.md` | new entry | |

## Order of work

1. Commit the intent, spec and plan.
2. Write the governance note, so the PreToolUse hook allows the edits.
3. Edit the nine files. (The widening edits to `rationale` and `acceptance-oracle-draft`'s description were made after the note was extended to name them.)
4. Update `docs/TASKS.md` and `crosscheck/JOURNAL.md`.
5. Run the checks below.

## Risks

- Open PR #72 (VA-1.1) edits nearby lines of `hellebuyck.md` and `intent-check/SKILL.md`. Whichever merges second resolves a text conflict.
- #76 merged and widened the VA-1.5 row with four strength claims (`/rationale`'s "Verified (static)", `/acceptance-oracle-draft`'s "measures whether the spec was the right spec", `/rationale`'s summary table and its "holds by construction" sentences). This change now fixes all four, so the row stays `done`.
- A `/rationale` consumer that matched `Verified (static)` or the `Verified` column breaks. A grep of `crosscheck/` and `docs/` finds no consumer.
- A reworded heading can break a reference. `## Rejected Flows` keeps its first two words, which `references/scenario-schema.md` cites.

## Proof

- NR-1: the grep in the spec prints only the keep list.
- NR-8: `go run ./crosscheck/conformance crosscheck` passes, and `git diff --stat` touches no file outside the table above.
- `node scripts/ci/task-queue.mjs check` and the tier gate tests pass.
