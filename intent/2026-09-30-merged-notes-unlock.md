# Intent: Stop merged governance notes from unlocking the protected-surface hook

Task: PB-1.3. Governing roadmap item: PB-1.

## Problem statement
The protected-surface hook, `.claude/hooks/protected-surface-guard.mjs`, allows an edit to a protected file when any note under `.assurance/protected-surface-amend/` or `.assurance/add-session-*/` names the file. It does not ask when the note was written. Notes are committed with the change they authorise and stay on `main` after the merge. So every file that any past note named stays unlocked for every later change, with no new note.

On `main` at 608ca86, the hook allows edits to 30 of the 58 tracked protected files with no new note. To check, feed the hook a `PreToolUse` payload for each tracked file that matches the machine-readable path list, and count the exits of 0. The task row says 29 of 57; #54 has since added one protected file and one note that names it.

The Tier Gate already refuses a note from an earlier change: "A note from an earlier change does not count" (`TIER-LAYER-MAP.md`, Tier 3). The hook is the local half of the same rule and does not apply it.

## Proposed outcome
- A note block unlocks an edit only if its text is not on the default branch. The default branch is `origin/HEAD`, or `origin/main` when `origin/HEAD` is not set. A block that is untracked, staged, or committed on the branch counts. A block that is on the default branch does not.
- Once a branch's note is squash-merged, the same text is on the default branch, so the note stops counting for that branch too.
- Adding a new block to an old note file, or editing an old block, unlocks only the files that the old note file did not already name. A block copied from any merged note, into any file, does not count.
- If the hook cannot find the default branch, it blocks edits to protected files and says how to fix it. Edits to other files are unaffected.
- On a clean checkout of `main`, the hook allows 0 of the protected files.
- The fallback contract in `/assurance-init`, and the documents that describe the hook, state the new rule.

## Affected users and systems
- Every agent and person editing this repository through Claude Code, since the hook runs before each Edit, Write and NotebookEdit.
- `.claude/hooks/protected-surface-guard.mjs`, and a new test file that the Tier Gate job runs.
- Repositories set up with `/assurance-init`. They copy the hook, or implement its contract from the SKILL.md when the copy is unreachable.
- `CLAUDE.md`, `.claude/rules/protected-surfaces.md`, `docs/assurance/DEVELOPMENT-FRAMEWORK.md` and `docs/gates/protected-surface-hook.md`, which describe the rule.
- `docs/TASKS.md`, whose rule "Until PB-1.3 is `done`, the protected-surface hook lets some edits through" is removed.

## Constraints
- The rules-file behaviour does not change: a missing rules file fails open, a malformed one fails closed.
- The path list, the directories scanned, and the substring match on the file path do not change.
- The hook adds no dependency. Git runs without a shell.
- The hook reads the default branch as last fetched. A note merged since the last `git fetch` still counts until the next fetch. It never counts for longer than that.
- The hook guards Edit, Write and NotebookEdit only. A write through the shell is not yet reached. The property that blocks it is that a shell command's writes are not known before it runs. The open question is whether a sandbox write rule on the protected paths can stand in for the hook there. This task does not change that.

## Open questions
None.
