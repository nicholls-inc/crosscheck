# Intent: The commit-msg hook reads every staged file name as git stores it

Task: PB-1.13. Governing roadmap item: PB-1.

## Problem statement
`.husky/commit-msg` blocks a `docs:` or `refactor:` commit that stages a behavioural artefact, a name that ends in `SKILL.md` or matches `agents/*.md`. It reads the staged names with `git diff --cached --name-only`, one per line, and greps them. Git C-quotes a name that holds a byte outside printable ASCII, a double quote, a backslash or a control character. The quoted name ends in `"`, so it fails the `$` anchor, and the commit goes through. A local run in a scratch repository, with `npx` stubbed so that only the type check runs and the message `docs: x`, gives:

```
crosscheck/skills/x/SKILL.md exit=1
crosscheck/skills/é/SKILL.md exit=0
crosscheck/agents/a\"b.md exit=0
crosscheck/skills/n$'\n'l/SKILL.md exit=0
```

## Proposed outcome
- The hook reads the names with `git diff --cached --name-only -z` and tests each one whole, so a quote, a non-ASCII byte or a newline in a name no longer hides it.
- A `docs:` or `refactor:` commit that stages any of the three names above fails, and the error lists the name as written.
- A test under `scripts/ci/`, which CI already runs, covers the hook for the first time.

## Affected users and systems
- Everyone who commits here with husky installed. A behavioural artefact under an unusual directory name now gets the same check as any other.
- `.husky/commit-msg`, a new `scripts/ci/commit-msg.test.mjs`, `docs/TASKS.md` and `JOURNAL.md`.

## Constraints
- The hook stays a POSIX `sh` script. It must run under dash (CI's Ubuntu `/bin/sh`) and macOS `sh`.
- `grep -z` is not portable. On this machine `grep` is ugrep, where `-z` decompresses input, and ugrep matched `crosscheck/skills/SKILL.md<newline>x`.
- No dependency is added. The commitlint step does not change.
- The hook runs only on a machine where husky is installed, and anyone can skip it with `--no-verify`. It is advisory, not a gate.

## Open questions
None. The task row states the outcome.
