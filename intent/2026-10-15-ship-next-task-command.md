# Intent: Commit the `/ship-next-task` command to the repository

## Problem statement
`/ship-next-task` picks up the next row of `docs/TASKS.md`, opens a PR for it, and review-triages the PR until pr-swarm approves HEAD and CI is green. It lives only in the maintainer's user config (`~/.claude/commands/ship-next-task.md`), and its gate script lives at `~/.claude/scripts/pr-gate.sh`. A fresh clone, a cloud session or a second machine has neither, so the loop that ships most tasks in this repository cannot run there, and a change to it leaves no trace in the history.

## Proposed outcome
- `.claude/commands/ship-next-task.md` holds the command, so Claude Code offers `/ship-next-task` in any checkout of this repository.
- `.claude/scripts/pr-gate.sh` holds the gate script, and the command calls it by that repo-relative path.
- The behaviour is unchanged. The command never merges, approves, arms auto-merge or force-pushes, and it states that pr-swarm's approving verdict is an LLM's judgement that makes the PR ready for the maintainer's review, not evidence that the change is correct.

## Affected users and systems
- The maintainer, and any session that runs `/ship-next-task` in this repository.
- The command depends on skills that this repository does not ship: `poteto-mode`, and `review-triage` and `pr-swarm` from the pr-swarm plugin. A checkout without them cannot run the command. That dependency is the same as before this change.

## Constraints
- No protected path changes. `.claude/commands/**` and `.claude/scripts/**` are not in the machine-readable list of `.claude/rules/protected-surfaces.md`.
- Tier 1: the change adds developer tooling and changes no output of Crosscheck or CGV.
- No guarantee rests on pr-swarm's verdict. The maintainer's merge stays the sign-off.

## Open questions
- Whether the gate script, which decides when the loop stops, should become a protected surface. It decides nothing about what merges, since the maintainer merges, so this change leaves it unprotected.
