# Intent: The conformance ledger rejects a claim status it does not know

Task: PB-1.20. Governing roadmap item: PB-1. Issue: #25. Decision: `intent/2026-10-07-backlog-review-decisions.md`, section "#25 → PB-1.20".

## Problem statement
`crosscheck/conformance/claims.json` gives each narrative claim a review `status`. The ledger loop in `crosscheck/conformance/main.go` acts on two values only. `unreviewed` is an error, and `known-gap` with no `tracked_in` is an error. Every other string passes. A claim written with `"status": "reviewed-disclsed"`, an empty status, or no `status` key at all passes the conformance job, which CI treats as blocking, and reads as a reviewed claim to anyone who skims the ledger. Measured on `origin/main`: a ledger of one claim with each of those three statuses makes `analyze` return no error.

## Proposed outcome
- The ledger accepts exactly four statuses: `unreviewed`, `known-gap`, `reviewed-disclosed` and `reviewed-accurate`. These are the two the loop already reads and the two that `claims.json` uses today.
- Any other status, an empty one, or a missing one is an error that names the claim, the status as written, and the four allowed values. The conformance job then exits 1.
- The header comment of `main.go` and `crosscheck/conformance/README.md` list the four statuses.
- A unit test covers a typo, an empty status, a missing status, a status with a leading space, and each of the four allowed values.

## Affected users and systems
- Anyone who adds or edits a claim in `claims.json`. A typo now fails the `conformance` job in `.github/workflows/ci.yml` instead of passing.
- `crosscheck/conformance/main.go`, `main_test.go` and `README.md`. `docs/TASKS.md`.
- The seven claims in `claims.json` today use only `reviewed-disclosed` and `reviewed-accurate`, so the real tree still passes.

## Constraints
- No protected surface changes. `crosscheck/conformance/` is not in `.claude/rules/protected-surfaces.md`.
- The existing `unreviewed` and `known-gap` errors keep their wording, so their tests do not change.
- Matching is exact. No trimming and no case folding, so `" reviewed-accurate"` and `Reviewed-Accurate` are unknown.

## Open questions
None. The decision record fixes the allowlist and the acceptance.

`loadLedger` returns no claims when `claims.json` is unreadable or is not valid JSON, so a syntax error in the ledger also makes every ledger check pass. `TestGoldenRealTree` catches that for this repository's own ledger, because it asserts seven claims, but the binary prints `RESULT: PASS`. That is a separate fault and gets its own row, PB-1.23.
