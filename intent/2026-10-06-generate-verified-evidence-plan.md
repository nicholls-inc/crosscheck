# Plan: `/generate-verified` emits an evidence record

Intent: `intent/2026-10-06-generate-verified-evidence.md`. Spec: `intent/2026-10-06-generate-verified-evidence-spec.md`. Task: ER-1.6. Tier 3, because `crosscheck/skills/generate-verified/SKILL.md` is a Class A protected surface.

## Files, in order

1. `.assurance/protected-surface-amend/generate-verified-evidence-2026-10-06.md`. The governance note naming `crosscheck/skills/generate-verified/SKILL.md`. It goes first, because the PreToolUse hook blocks the edit until a note new on the branch names the file.
2. `crosscheck/skills/generate-verified/SKILL.md`.
   - Front matter: `argument-hint` names `evidence:` and `requirement:`.
   - Step 6 keeps only the write of `impl.dfy`. A new "Step 7: Emit an Evidence Record" states GV-1 to GV-8 as instructions, with the exact `git` commands and the `dafny_evidence` arguments.
   - The Evidence Summary moves to a new Step 8, so it reports Step 7's outcome. Its template gains the record lines and the unticked statement review item of GV-9.
   - "Arguments" documents `evidence:` and `requirement:`, with an example.
3. `crosscheck/docs/skills.md`. The `/generate-verified` row says it can emit an evidence record.
4. `crosscheck/README.md`. The `dafny_evidence` row names `/generate-verified` as its caller.
5. `crosscheck/JOURNAL.md`. An entry for the decision.
6. `docs/TASKS.md`. ER-1.6 is `done`, with the intent as its record.

## Risks

- **A commit in the caller's repository.** GV-1 makes it opt-in, GV-4 refuses a dirty tree before it, and GV-5 commits one path and never skips hooks.
- **The skill and the tool drift.** The skill restates no DE rule. It names the tool's arguments and reports its errors, so a change to DE-1 to DE-13 needs no change here unless the arguments change.
- **`.crosscheck/.gitignore` surprises a user.** It is written only on the opt-in path and only when git does not already ignore the directory, and the Evidence Summary names it.

## Proof

- Follow Step 7 by hand on a scratch repository against the real `crosscheck-dafny:latest` image, through the MCP server built from this branch, for each case in the spec's Verification section.
- `node scripts/check-evidence-record.mjs` exits 0 on the record, and the record's rerun command exits 0 under `sh`.
- `node scripts/ci/task-queue.mjs check` passes, and the tier gate passes at Tier 3 with this plan cited.
