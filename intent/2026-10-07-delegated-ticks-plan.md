# Plan: Delegated ticks and mechanical checklist items

Intent: `intent/2026-10-07-delegated-ticks.md`
Spec: `intent/2026-10-07-delegated-ticks-spec.md` (DT-1 to DT-7)
Governing roadmap item: PB-1. Task: PB-1.38. Tier: 3.

This plan is not the root `plan.md`, which belongs to an earlier change. The pull request body cites this file with a `Plan:` line.

## Order of work

1. Write the governance note `.assurance/protected-surface-amend/delegated-ticks-2026-10-07.md`, naming the four protected files below. Commit it alone.
2. Edit `.claude/rules/protected-surfaces.md`: Amendment pattern steps 1 and 4, and a new paragraph for mechanical items (DT-1 to DT-6).
3. Edit `crosscheck/skills/protected-surface-amend/SKILL.md`: the intro paragraph and the Review Checklist template, so mechanical items are drafted ticked with evidence (DT-5, DT-6).
4. Edit `crosscheck/skills/assurance-init/SKILL.md`: the Step 5 template's Amendment pattern (DT-7).
5. Edit `REVIEW.md` and `docs/gates/protected-surface-amendment.md` to match.
6. Add `docs/TASKS.md` rows: PB-1.38 `done`, PB-1.39 `todo`. Add a root `JOURNAL.md` entry.
7. Run `node --test scripts/ci/*.test.mjs` and `node scripts/ci/skill-references.mjs` if present.

## Files

| File | Protected | Change |
|---|---|---|
| `.claude/rules/protected-surfaces.md` | yes (`.claude/rules/**`) | DT-1 to DT-6 |
| `crosscheck/skills/protected-surface-amend/SKILL.md` | yes (`crosscheck/skills/*/SKILL.md`) | DT-5, DT-6 |
| `crosscheck/skills/assurance-init/SKILL.md` | yes (`crosscheck/skills/*/SKILL.md`) | DT-7 |
| `REVIEW.md` | no | one sentence |
| `docs/gates/protected-surface-amendment.md` | no | decision section |
| `docs/TASKS.md` | no | PB-1.38 `done`, new row PB-1.39 |
| `JOURNAL.md` | no | entry |
| `.assurance/protected-surface-amend/delegated-ticks-2026-10-07.md` | no | new governance note |
| `intent/2026-10-07-delegated-ticks*.md` | no | stage artefacts |

## Risks

- **A drafting tool ticks a mechanical item it did not run.** Mitigation: DT-5 requires the command and its output under the item; a reviewer sees an item with no output and unticks it. A check that recomputes them is PB-1.39.
- **An agent reads DT-1 as permission to tick unprompted.** Mitigation: DT-2 says so in the rule, and the instruction must come from the human the agent works for.
- **The commit type.** The change touches two `SKILL.md` files, so the commit is `feat(crosscheck):`, not `docs:` or `refactor:`.
