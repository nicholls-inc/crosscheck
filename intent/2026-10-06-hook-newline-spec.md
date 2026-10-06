# Spec: The protected-surface hook matches a path that holds a newline

Intent: `intent/2026-10-06-hook-newline.md`. Governing roadmap item: PB-1. Task: PB-1.14.

This spec adds PG-9 to `intent/2026-09-30-merged-notes-unlock-spec.md` and narrows PG-6, which named the glob matcher as unchanged. The other PG requirements do not change.

- **PG-9. Newlines in a path.** The hook compiles each glob in the machine-readable path list with the `s` flag. `**` matches any run of characters, newlines included. `*` and `?` still match no `/`, and they already matched a newline. So `docs/assurance/**` matches `docs/assurance/n<newline>l.md`, and an edit to it with no new governance note exits 2 with the gate message. A path with no newline matches the same globs as before, since the flag changes only what `.` matches.
- **PG-9 tests.** `scripts/ci/protected-surface-guard.test.mjs`, with the rule `protected/**`, adds:
  - an edit to `protected/n<newline>l.txt` with no note: exit 2, and stderr is the gate message;
  - an edit to `protected/x<newline>y/n<newline>l.txt` with no note: exit 2, and stderr is the gate message;
  - an edit to `protected/n<newline>l.txt`: exit 2 with no note, then exit 0 once an untracked note names it.
- **Known gap, not a rule.** The hook sees the path that the harness passes in `tool_input.file_path`. The tests feed that payload directly. They do not drive a real harness edit of a file whose name holds a newline.
