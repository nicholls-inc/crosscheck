# Intent: The Tier Gate reads every changed file name as git stores it

Task: PB-1.10. Governing roadmap item: PB-1.

## Problem statement
The `Run tier gate` step in `.github/workflows/tier-gate.yml` sets `CHANGED_FILES` from `git diff --name-only --no-renames`, one name per line. Git C-quotes a name that holds a byte outside printable ASCII, a double quote, a backslash or a control character. The quoted name starts with `"`, so it matches no protected glob, and `docs/assurance/é.md` changed at `Tier: 1` passes the gate. A local run shows the quoting:

```
"docs/assurance/a\"b.md"
"docs/assurance/n\nl.md"
"docs/assurance/\303\251.md"
```

`git -c core.quotePath=false` stops quoting only the non-ASCII name. The other two stay quoted. A newline-separated list cannot carry a name that holds a newline in any case.

## Proposed outcome
- The step writes `git diff -z --name-only --no-renames` to a temporary file and passes its path to the gate as `CHANGED_FILES_PATH`. The gate splits the file on NUL and keeps each name exactly as git wrote it.
- `CHANGED_FILES` is removed. The workflow is its only caller.
- A pull request that changes `docs/assurance/é.md`, `docs/assurance/a"b.md` or `docs/assurance/n<newline>l.md` at `Tier: 1` fails on the Tier 3 floor, and the failure names the file.

## Affected users and systems
- Everyone who opens a pull request here. A file name with one of those bytes now counts against the protected globs.
- `.github/workflows/tier-gate.yml`, `scripts/ci/tier-gate.mjs`, `scripts/ci/tier-gate-workflow.test.mjs`.
- `intent/2026-09-29-deterministic-evidence-spec.md` (the gate's inputs), `docs/TASKS.md` and `JOURNAL.md`.

## Constraints
- No dependency is added.
- A failed `git diff` still fails the step under `bash -e`.
- The gate's other inputs (`PR_BODY`, `PR_LABELS`, `BASE_REF`, `CROSSCHECK_PROTECTED_RULES`) do not change.

## Open questions
None. The task row states the outcome.

`.husky/commit-msg` reads `git diff --cached --name-only` the same way, so a `SKILL.md` under a directory with a non-ASCII name escapes its commit-type check. That is a separate surface and gets its own row, PB-1.13.
