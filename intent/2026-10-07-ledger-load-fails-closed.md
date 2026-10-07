# Intent: The conformance ledger fails when `claims.json` cannot be read or parsed

Task: PB-1.23. Governing roadmap item: PB-1. Found in PB-1.20 (`intent/2026-10-07-ledger-status-allowlist.md`, last paragraph).

## Problem statement
`loadLedger` in `crosscheck/conformance/main.go` returns no claims, and no error, in three cases: the file is missing, the file cannot be read, and the file is not valid JSON for the ledger shape. The ledger checks then loop over nothing, so a syntax error in `claims.json` or a file the job cannot open makes every ledger check pass. The `conformance` job in `.github/workflows/ci.yml` treats the run as blocking, so it passes.

Measured on `origin/main` at 7cafa0d, from `crosscheck/conformance` with `go run . ..`:

- with `{"narrative_claims": [` appended to `claims.json`, the run prints `NARRATIVE LEDGER (0 claims):`, `RESULT: PASS`, and exits 0;
- with `claims.json` at mode 000, the run prints the same and exits 0.

`TestGoldenRealTree` asserts seven claims, so it catches a broken ledger in this repository's own tree. The binary does not, and the binary is what the job runs.

## Proposed outcome
- A `claims.json` that exists but cannot be read is an error that names the file and the read error.
- A `claims.json` that is read but does not decode into the ledger shape is an error that names the file and the decode error. An empty file is in this case.
- A missing `claims.json` stays an empty ledger with no error, as the task row allows.
- An error from either case makes the run print `RESULT: FAIL` and exit 1.
- Tests cover a missing file, an unreadable file, and invalid JSON.

## Affected users and systems
- Anyone who edits `claims.json`. A syntax error now fails the `conformance` job instead of passing with zero claims.
- `crosscheck/conformance/main.go`, `main_test.go` and `README.md`. `docs/TASKS.md`.
- The real `claims.json` is valid, so the real tree still passes with seven claims.

## Constraints
- No protected surface changes. `crosscheck/conformance/` is not in `.claude/rules/protected-surfaces.md`.
- The PB-1.20 status checks and the other ledger checks keep their messages.

## Open questions
None. The task row fixes the outcome, including that a missing file stays an empty ledger.
