# Spec: The conformance ledger rejects a claim status it does not know

Intent: `intent/2026-10-07-ledger-status-allowlist.md`. Governing roadmap item: PB-1. Task: PB-1.20.

- **LS-1. Allowlist.** A narrative claim's `status` is valid when it equals, byte for byte, one of `unreviewed`, `known-gap`, `reviewed-disclosed` or `reviewed-accurate`. `crosscheck/conformance/main.go` holds the four values in one map, `knownStatus`.
- **LS-2. Unknown status.** For each claim whose status is not valid, `analyze` appends one error: `[ledger] claim <id> has unknown status "<status>" (want one of unreviewed|known-gap|reviewed-disclosed|reviewed-accurate)`. The status is printed with Go's `%q`, so an empty status shows as `""` and a leading space stays visible. A claim with no `status` key decodes to the empty string and gets the same error.
- **LS-3. Existing rules stand.** The `unreviewed` error, the `known-gap` without `tracked_in` error and the `present_artifact` check run as before, and with the same messages. An unknown status does not skip the `present_artifact` check.
- **LS-4. Exit code.** An LS-2 error is an ordinary error, so the run exits 1 and prints `RESULT: FAIL`.
- **LS-5. Documentation.** The header comment of `main.go` lists the four statuses with one line on each, and says that any other status is an error. `crosscheck/conformance/README.md` says the same in its LEDGER paragraph, its exit-code line and its "Extend" section.
- **LS-6. Tests.** `TestLedgerStatusAllowlist` in `main_test.go` builds a tree with one claim and asserts the LS-2 message for `reviewed-disclsed`, `""`, a missing key and `" reviewed-accurate"`, and no `unknown status` error for each of the four allowed values. Removing any one value from `knownStatus` fails the test, and so does trimming the status or exempting the empty one.
- **Known gaps, not rules.**
  - An unreadable or invalid `claims.json` still yields no claims and no error. PB-1.23 covers it.
  - The allowlist is checked by the Go oracle only. Nothing checks the schema of `claims.json` in an editor.
