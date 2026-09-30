# Plan: Stop merged governance notes from unlocking the protected-surface hook

Intent: `intent/2026-09-30-merged-notes-unlock.md`
Spec: `intent/2026-09-30-merged-notes-unlock-spec.md` (PG-1 to PG-8)
Governing roadmap item: PB-1. Task: PB-1.3. Tier: 3.

This plan is not the root `plan.md`, which belongs to an earlier change. The pull request body cites this file with a `Plan:` line.

## Order of work

1. Write the governance note `.assurance/protected-surface-amend/merged-notes-unlock-2026-09-30.md`, naming the five protected files below.
2. Write `scripts/ci/protected-surface-guard.test.mjs` (PG-7). Run it and see the merged-note cases fail against the current hook.
3. In `.claude/hooks/protected-surface-guard.mjs`, resolve the default-branch commit (PG-1), read each note file's text at that commit with `git show <sha>:<path>`, and count only new blocks (PG-2, PG-3). Block a protected edit when no default branch resolves (PG-4). Reword the gate message's reason clause (PG-6).
4. Run `node --test scripts/ci/*.test.mjs`.
5. On a clean checkout of `main` with this hook, feed a payload for every tracked protected file and check that 0 of them exit 0. Check that the same count on the branch, with this plan's note present, is exactly the five files the note names.
6. Update the documents in PG-8.
7. Set PB-1.3 to `done` in `docs/TASKS.md`, with this intent as its record, and add a root `JOURNAL.md` entry.

## Files

| File | Protected | Change |
|---|---|---|
| `.claude/hooks/protected-surface-guard.mjs` | yes (`.claude/hooks/**`) | count only new blocks (PG-1 to PG-6) |
| `scripts/ci/protected-surface-guard.test.mjs` | yes (`scripts/ci/**`) | new (PG-7) |
| `.claude/rules/protected-surfaces.md` | yes (`.claude/rules/**`) | deterministic-layer wording (PG-8) |
| `docs/assurance/DEVELOPMENT-FRAMEWORK.md` | yes (`docs/assurance/**`) | stage 3 wording (PG-8) |
| `crosscheck/skills/assurance-init/SKILL.md` | yes (`crosscheck/skills/*/SKILL.md`) | step 7.5b hook contract (PG-8) |
| `CLAUDE.md` | no | wording (PG-8) |
| `docs/gates/protected-surface-hook.md` | no | new rule and the PG-4 block (PG-8) |
| `docs/TASKS.md` | no | PB-1.3 `done`, PB-1.3 rule removed |
| `JOURNAL.md` | no | entry |
| `.assurance/protected-surface-amend/merged-notes-unlock-2026-09-30.md` | no | new governance note |
| `intent/2026-09-30-merged-notes-unlock*.md` | no | stage artefacts |

The test lives in `scripts/ci/` so that the Tier Gate job's existing `node --test scripts/ci/*.test.mjs` runs it. No workflow changes.

## Risks

- **A stale `origin/main`.** A note merged since the last fetch still counts until the next fetch. This is stated in the intent's constraints. The Tier Gate, which diffs the pull request, still refuses such a note in CI.
- **A clone with no remote.** Every protected edit is blocked until `origin` exists and is fetched. That is the conservative choice the hook already makes for failures other than a missing rules file. The message names the fix.
- **A block copied verbatim from an old note.** It is not new, so it does not count. That fails closed.
- **The commit type.** The SKILL.md edit makes this a `fix(crosscheck):` commit, as `CLAUDE.md` requires for a behavioural artefact.
- **This change locks its own follow-ups.** Once merged, this pull request's note stops counting, so a later edit to the hook needs a new note. That is the intended outcome.

## Proof that it worked

- `node --test scripts/ci/*.test.mjs` passes, including every PG-7 case.
- Step 5 prints 0 on `main`. On the branch it also lists `docs/assurance/ROADMAP.md`, beyond the five files the note names. The note's "Governing Roadmap Item" section mentions that path, and the hook counts any mention of a path in a block (PG-6 keeps that rule). So every new note unlocks the roadmap. Closing that is a follow-up task.
