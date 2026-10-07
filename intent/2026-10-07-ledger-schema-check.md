# Intent: The conformance run fails on a `claims.json` of the wrong shape

Task: PB-1.25. Governing roadmap item: PB-1. Found in PB-1.23 (`intent/2026-10-07-ledger-load-fails-closed-spec.md`, known gaps).

## Problem statement
`loadLedger` in `crosscheck/conformance/main.go` decodes `claims.json` with `json.Unmarshal` into the ledger structs. `json.Unmarshal` accepts any JSON that fits the structs. It ignores keys the structs do not name, matches key names without regard to case, and leaves the structs empty for a top-level `null` or `{}`. So a ledger of the wrong shape loads as an empty or partial ledger, every ledger check loops over nothing or over part of the claims, and the run passes.

Measured on `origin/main` at 904d948, from `crosscheck/conformance` with `go run . ..`, after replacing `claims.json` with each of `null`, `{}`, `{"version":1}`, `{"narrative_claims":[],"bogus":1}` and `{"Narrative_Claims":[]}`: every run prints `ERRORS   : 0`, `NARRATIVE LEDGER (0 claims):` and `RESULT: PASS`.

A misspelt key inside a claim is worse. A claim whose `"status"` is spelt `"staus"` decodes with an empty status, which the allowlist catches. But a claim whose `"tracked_in"` is spelt `"tracked-in"` or whose `"expect_present"` is spelt `"expect-present"` decodes as a claim with no link or with the default expectation, and nothing reports the key that was dropped.

## Proposed outcome
- The ledger schema is the shape the real `claims.json` and the Go types already have. The top level is an object with the keys `version`, `description` and `narrative_claims`. `narrative_claims` is an array of claim objects with the keys `id`, `source`, `claim`, `reality`, `status`, `check` and `tracked_in`. `check` is an object with the keys `type`, `path` and `expect_present`. Key names match exactly, including case.
- A `claims.json` that parses but breaks that schema is an LL-3 error, `[ledger] cannot parse conformance/claims.json: <reason>`, and the run prints `RESULT: FAIL` and exits 1. The cases are a top-level value that is not an object, including `null`; a missing or `null` `narrative_claims`; a claim or a `check` that is not an object, including `null`; and a key the schema does not name at any of the three levels.
- The real `claims.json` still loads its seven claims and the real tree still passes.
- `crosscheck/conformance/README.md` and the header comment of `main.go` no longer list the wrong shape as not yet reached, and name what is still not checked.

## Affected users and systems
- Anyone who edits `claims.json`. A misspelt or unknown key now fails the `conformance` job instead of being dropped.
- `crosscheck/conformance/main.go`, `main_test.go` and `README.md`. The spec `intent/2026-10-07-ledger-load-fails-closed-spec.md`. `docs/TASKS.md`.

## Constraints
- No protected surface changes. `crosscheck/conformance/` is not in `.claude/rules/protected-surfaces.md`.
- The LL-2 and LL-3 messages keep their prefixes. A schema error is a parse error, so it uses the LL-3 prefix.
- Field types stay as `json.Unmarshal` checks them today.
- Required fields inside a claim (a non-empty `id`, a `version` value), an allowlist for `check.type`, and duplicate keys are separate work, queued as PB-1.40. This change checks which keys may appear and which values must be objects, not which keys must appear inside a claim.

## Open questions
None. README named one: which schema the ledger should be held to. The answer here is the schema the real file and the Go types already share, with exact key names, so no existing ledger changes meaning. A stricter schema with required fields is PB-1.40.
