# Spec: The Tier Gate fails closed without its changed-file list

Intent: `intent/2026-10-06-changed-files-fail-closed.md`. Governing roadmap item: PB-1. Task: PB-1.15.

This spec adds TG-17 and revises TG-16 of `intent/2026-10-06-unquoted-paths-spec.md`, which kept the fail-open read of an unset `CHANGED_FILES_PATH` for this task. The other TG requirements do not change.

- **TG-16, revised.** The sentence "With `CHANGED_FILES_PATH` unset, the list is empty" is replaced by TG-17. The rest of TG-16 stands.
- **TG-17. Fail closed.** When the gate runs as a script:
  - if `CHANGED_FILES_PATH` is unset or the empty string, it prints the failure lines of TG-10 with one item that names `CHANGED_FILES_PATH` and says how to write the list, and exits 1;
  - if reading the file that `CHANGED_FILES_PATH` names throws, it prints the failure lines of TG-10 with one item that names `CHANGED_FILES_PATH`, the path and the error code, and exits 1. It prints no stack trace;
  - in both cases it does not evaluate the tier, so no `tier-gate: PASS` line is printed;
  - an empty readable file is an empty list, as before.
- **TG-18. Tests.** `scripts/ci/tier-gate.test.mjs` runs the script as a child process. The body is `Tier: 1` with the citation `Intent: intent/a.md`, and the scratch repository holds `intent/a.md` and the protected file `scripts/ci/x.mjs`. The cases are:
  - `CHANGED_FILES_PATH` unset: exit 1, the output carries the TG-10 heading, names `CHANGED_FILES_PATH` and the `git diff -z --name-only` command, and holds no `tier-gate: PASS`. The old gate passed this case at Tier 1;
  - `CHANGED_FILES_PATH` empty: the same;
  - `CHANGED_FILES_PATH` naming a missing file: exit 1, the output carries the TG-10 heading and names `CHANGED_FILES_PATH`, the path and `ENOENT`, and it holds no stack frame (`    at `);
  - `CHANGED_FILES_PATH` naming a directory: exit 1, the output names `CHANGED_FILES_PATH`, the path and `EISDIR`, and holds no stack frame;
  - `CHANGED_FILES_PATH` naming an empty readable file: exit 0 and a `tier-gate: PASS` line at Tier 1, since an empty file is an empty diff;
  - `CHANGED_FILES_PATH` naming a readable file that lists `intent/a.md`: exit 0 and a `tier-gate: PASS` line at Tier 1, so the failures above come from the variable and not from the scratch repository;
  - `CHANGED_FILES_PATH` naming a readable file that also lists `scripts/ci/x.mjs`: exit 1 on the Tier 3 floor, so the gate reads the list it is given.
- **Known gaps, not rules.**
  - The gate cannot tell a file the workflow wrote from a file a caller wrote by hand, so a hand-written list that omits a protected path still passes. The workflow step that writes the list is TG-14's.
