# Spec: The conformance ledger fails when `claims.json` cannot be read or parsed

Intent: `intent/2026-10-07-ledger-load-fails-closed.md`. Governing roadmap item: PB-1. Task: PB-1.23.

- **LL-1. Missing file.** When reading `<root>/conformance/claims.json` fails with an error that `errors.Is(err, fs.ErrNotExist)` accepts, the ledger is empty and `analyze` adds no ledger error.
- **LL-2. Unreadable file.** When the read fails with any other error, `analyze` appends one error, `[ledger] cannot read conformance/claims.json: <error>`, and the ledger is empty. A path that is a directory and a file without read permission are both in this case.
- **LL-3. Invalid content.** When the file is read but `json.Unmarshal` into the ledger shape fails, `analyze` appends one error, `[ledger] cannot parse conformance/claims.json: <error>`, and the ledger is empty. An empty file, truncated JSON, and a `narrative_claims` value that is not an array are all in this case.
- **LL-4. Exit code.** An LL-2 or LL-3 error is an ordinary error, so `report` prints `RESULT: FAIL` and the run exits 1.
- **LL-5. Existing rules stand.** A ledger that decodes runs the status, `unreviewed`, `known-gap` and `present_artifact` checks as before, with the same messages.
- **LL-6. Documentation.** The header comment of `main.go` and `crosscheck/conformance/README.md` say that a missing `claims.json` is an empty ledger and that one that cannot be read or parsed is an error.
- **LL-7. Tests.** `TestLedgerLoad` in `main_test.go` builds a tree per case and asserts the error for a directory at the path, a mode-000 file (skipped when the process runs as root, which can read it), truncated JSON, an empty file, and a non-array `narrative_claims`, and asserts that each of them makes `report` print `RESULT: FAIL`. It asserts no ledger error for a missing file. Restoring the old `loadLedger`, which returns no claims on any failure, fails every error case.
- **Known gaps, not rules.**
  - A `claims.json` that is a symlink to a missing target reads as missing under LL-1, so it is an empty ledger.
  - JSON that decodes but carries unknown keys, or a top-level `null` or `{}`, is a valid empty or partial ledger. Nothing checks the schema of `claims.json`.
