# Intent: The protected-surface hook matches a path that holds a newline

Task: PB-1.14. Governing roadmap item: PB-1.

## Problem statement
`.claude/hooks/protected-surface-guard.mjs` compiles each protected glob to a regular expression with no flags. Its `globToRegExp` turns `**` into `.*`, and `.` without the `s` flag matches no line terminator. So `docs/assurance/**` does not match `docs/assurance/n<newline>l.md`, and the hook lets an edit to that file through with no governance note. A local run on `main` shows it:

```
docs/assurance/a.md exit=2
docs/assurance/n$'\n'l.md exit=0
docs/assurance/x/n$'\n'l.md exit=0
crosscheck/skills/x$'\n'y/SKILL.md exit=2
```

`*` and `?` compile to `[^/]*` and `[^/]`, which match a newline, so only `**` has the fault. The tier gate had the same fault and compiles with the `s` flag since PB-1.10.

## Proposed outcome
- `globToRegExp` in the hook compiles with the `s` flag, so `**` matches any characters, newlines included.
- An edit to `docs/assurance/n<newline>l.md` with no new governance note exits 2 with the gate message.

## Affected users and systems
- Every agent that edits files here. A path with a newline under a `**` glob is now protected like any other path.
- `.claude/hooks/protected-surface-guard.mjs`, `scripts/ci/protected-surface-guard.test.mjs`.
- `intent/2026-09-30-merged-notes-unlock-spec.md` (PG-6 named the glob matcher as unchanged), `docs/TASKS.md` and `JOURNAL.md`.

## Constraints
- No dependency is added.
- A path with no newline matches exactly the globs it matched before.

## Open questions
None. The task row states the outcome.
