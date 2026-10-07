# Intent: A dangling `claims.json` symlink fails the conformance run

Task: PB-1.24. Governing roadmap item: PB-1. Found in PB-1.23 (`intent/2026-10-07-ledger-load-fails-closed-spec.md`, known gaps).

## Problem statement
`loadLedger` in `crosscheck/conformance/main.go` treats a `claims.json` that `os.ReadFile` reports as missing as an empty ledger (LL-1). `os.ReadFile` follows a symbolic link, so a `claims.json` that is a symlink to a missing target is reported as missing too. The ledger loads with no claims and no error, every ledger check loops over nothing, and the run passes.

Measured on `origin/main` at 0f6a839, from `crosscheck/conformance` with `go run . ..`, after replacing `claims.json` with a symlink to `does-not-exist.json`: the run prints `ERRORS   : 0`, `NARRATIVE LEDGER (0 claims):` and `RESULT: PASS`.

A dangling symlink is a ledger someone put in place and broke, not a ledger nobody wrote. The same holds for a `conformance` directory that is a symlink to a missing target.

## Proposed outcome
- A `claims.json` that is a symlink to a missing target is an LL-2 error: `[ledger] cannot read conformance/claims.json: ...`, and the run prints `RESULT: FAIL` and exits 1.
- The same holds when `conformance` under the plugin root is a symlink to a missing target.
- A `claims.json` that does not exist, with no link in its place, stays an empty ledger.
- A symlink to a readable ledger loads as before.
- `crosscheck/conformance/README.md` and the header comment of `main.go` no longer list the dangling link as not yet reached.

## Affected users and systems
- Anyone who replaces `claims.json` with a link. A broken link now fails the `conformance` job instead of passing with zero claims.
- `crosscheck/conformance/main.go`, `main_test.go` and `README.md`. The spec `intent/2026-10-07-ledger-load-fails-closed-spec.md`. `docs/TASKS.md`.
- The real `claims.json` is a regular file, so the real tree still passes with seven claims.

## Constraints
- No protected surface changes. `crosscheck/conformance/` is not in `.claude/rules/protected-surfaces.md`.
- The LL-2 and LL-3 messages keep their prefixes.
- PB-1.25 (schema check) is a separate row and stays out of this change.

## Open questions
None. The task row settles the open question that PB-1.23's README named: a dangling link is an unreadable ledger, not a missing one.
