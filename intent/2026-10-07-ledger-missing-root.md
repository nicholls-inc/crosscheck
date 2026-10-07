# Intent: A plugin root that does not resolve fails the conformance run

Task: PB-1.30. Governing roadmap item: PB-1. Found in the PB-1.24 review (`intent/2026-10-07-ledger-dangling-symlink.md`).

## Problem statement
PB-1.24 made `loadLedger` in `crosscheck/conformance/main.go` fail when `conformance/claims.json` or `conformance` is a symlink to a missing target (LL-8). The check runs `os.Lstat` on those two paths only. When the plugin root itself does not resolve, `os.Lstat` fails on both, so neither counts as a dangling link, and the read error falls through to LL-1: an empty ledger with no error.

The root fails to resolve when it is a symlink to a missing target, when an ancestor of it is, or when it does not exist at all. The task row asked whether `analyze` already fails on a missing root. It does not. Measured on `origin/main` at 6116404, from `crosscheck/conformance`, with `go run . <root>`:

- `<root>` a path that does not exist: `skills discovered : 0`, `ERRORS   : 0`, `NARRATIVE LEDGER (0 claims):`, `RESULT: PASS`, exit 0.
- `<root>` a symlink to a missing target: the same output, exit 0.
- `<root>` = `<link>/crosscheck`, where `<link>` is a symlink to a missing target: the same output, exit 0.

A run that scans nothing cannot vouch for anything, so it must not print `RESULT: PASS`.

## Proposed outcome
- When the read of `claims.json` reports a missing file, no LL-8 link applies, and `os.Stat` on the plugin root fails, `analyze` appends one error, `[ledger] cannot read conformance/claims.json: plugin root <root> does not resolve: <error>`. The run prints `RESULT: FAIL` and exits 1. This covers a missing root, a dangling root link, and a dangling link in an ancestor of the root.
- A root that resolves to a directory with no `conformance/claims.json` stays an empty ledger, as LL-1 says. A root that is a symlink to a real plugin tree loads as before.
- `TestLedgerLoad` asserts, for each dangling-link case, and `TestLedgerLoadRoot` asserts, for each failing root case, that the error from `loadLedger` wraps the original with `errors.Is(err, fs.ErrNotExist)`.
- The header comment of `main.go` names the dangling `conformance` link and the unresolved root. `crosscheck/conformance/README.md` names the unresolved root.

## Affected users and systems
- Anyone who runs `go run ./crosscheck/conformance <root>` with a wrong path. The run now fails instead of passing with zero skills, agents and claims.
- `crosscheck/conformance/main.go`, `main_test.go` and `README.md`. The spec `intent/2026-10-07-ledger-load-fails-closed-spec.md`. `docs/TASKS.md`.
- The `conformance` CI job runs on the real `crosscheck/` tree, which resolves, so it still passes with seven claims.

## Constraints
- No protected surface changes. `crosscheck/conformance/` is not in `.claude/rules/protected-surfaces.md`.
- The LL-2, LL-3 and LL-8 messages keep their prefixes.
- PB-1.25 (schema check) is a separate row and stays out of this change.

## Open questions
The measurement above answers the open question in the task row: `analyze` does not fail on a missing root, so the check belongs in this change.

Not yet reached: a root that exists but is not a plugin tree, such as an empty directory, still passes with 0 skills. The blocking property is that nothing marks a directory as a plugin root. The open question is what does. Queued as PB-1.41, with tests for the roots that fail through LL-2 (a regular file, an unreadable directory, a symlink loop).
