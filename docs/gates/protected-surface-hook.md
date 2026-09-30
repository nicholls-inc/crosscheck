# Gate: Protected-Surface Hook (PreToolUse Block)

## What this gate protects

Some files in this repository are **protected surfaces**: paths that define how Crosscheck's skills and agents behave, or that encode the correctness contracts ("invariants") those skills check against — for example a skill's `SKILL.md`, an agent's prompt, a file under `docs/invariants/`, or the hooks and rules that enforce all of this. A silent, unreviewed edit to one of these could quietly change what Crosscheck verifies, or how, without anyone noticing until much later.

To stop that, a deterministic **PreToolUse hook** (`.claude/hooks/protected-surface-guard.mjs`) runs before any file edit. A PreToolUse hook is a script the agent harness always runs immediately before a tool call is allowed to execute — unlike a skill, which only makes good behaviour *likely*, a hook makes it *certain*. This one checks every edit against the machine-readable path list in `.claude/rules/protected-surfaces.md`. If the edited file matches a protected path, the hook looks for a **governance-note block**: a structured record, produced by the `/protected-surface-amend` skill, that names the file and documents why the change is authorised. The note must be new on this branch, not already on the default branch. No such note, no edit — the hook exits with a block status and the edit does not happen.

## What you are being asked to decide

You (or the agent acting on your behalf) attempted to edit a protected file and the edit was stopped before it touched disk. You are being asked to choose how to proceed: get the file properly authorised, or leave it unchanged.

## What each decision means

- **Approving (running `/protected-surface-amend` first)**: run the `/protected-surface-amend` skill against the file you intended to edit. It drafts the governance-note block — recording what is changing, why, under what authority, and what a reviewer must check — and writes it to `.assurance/protected-surface-amend/`. The block must be new on this branch. Once that block exists and names your file, re-run the same edit; the hook will find the block and allow it through.
- **Declining**: leave the file as it is. Nothing is lost — the hook only blocked the write, it did not alter or damage anything already in the working tree.

There is no third option that skips authorisation. The hook does not accept an override flag, an environment variable, or a differently worded governance note — only a genuine block naming the exact file.

## Why the hook fails open when the rules file is absent

The hook reads its policy from `.claude/rules/protected-surfaces.md`. If that file does not exist at all, the hook allows the edit rather than blocking it. This is a deliberate, narrow exception, not a loophole: a repository that has not yet adopted this protection (or is mid-bootstrap, before the rules file has been created) must not have every single edit bricked by a policy file it doesn't have yet. This is different from every other failure the hook might hit — an unreadable amendment file, a malformed glob, a missing repository root — all of which are treated conservatively (the hook still blocks). Only a genuinely absent rules file is read as "this repository has not opted in," and only that case fails open. If you see edits going through unexpectedly, check first whether `.claude/rules/protected-surfaces.md` exists; if it does, the hook is applying its rules and the missing-file exception does not apply.

## Why a note already on the default branch does not count

After a governance note is merged to the default branch, it stays there. If the hook counted notes that already exist on main, every file that any past note named would unlock forever — the note becomes a standing permission. To prevent that, the hook compares each governance-note block on this branch against every note file on the default branch. If the block's text is already there, in any note file, it is not new, and it unlocks nothing, even if it has been copied into a new file. A new or edited block in an old note file unlocks only the files that note file did not already name, so editing an old note cannot renew what it once unlocked. To change a file an old note already named, write a new note file, which `/protected-surface-amend` does anyway. The check catches exact copies only: a copied block changed by even one character counts as new, the same as a freshly written note, and the reviewer of the pull request is the check on it. In CI, the Tier Gate applies a weaker rule: a governance note changed in the pull request must name each changed protected file. It does not compare block text with the default branch. The hook reads the default branch as last fetched, so a note merged since your last `git fetch origin` still counts until you fetch again.

## When the hook cannot find the default branch

The hook resolves the default branch in order: first `origin/HEAD`, then `origin/main`. If neither reference exists (for example, in a repository with no `origin` remote, or one that was never fetched), the hook cannot determine whether a note is new and blocks all protected edits to be safe. The fix is to run `git fetch origin` followed by `git remote set-head origin --auto`, which tells git what the remote's default branch is.

## How long this takes

Running `/protected-surface-amend` and re-attempting the edit typically takes a few minutes, since the skill drafts the governance-note block automatically from the diff, commit history, and the repository's roadmap; you mainly need to confirm the drafted rationale and authoriser are accurate.

## Who to ask if unsure

The Crosscheck maintainers, via a GitHub issue on this repository.
