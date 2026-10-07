# Intent: `claims.json` must carry its required fields, allowed values and no duplicate keys

Task: PB-1.40. Governing roadmap item: PB-1. Found in PB-1.25 (`intent/2026-10-07-ledger-schema-check.md`, constraints).

## Problem statement
PB-1.25 made `loadLedger` in `crosscheck/conformance/main.go` check which keys may appear in `claims.json`. It does not check which keys must appear, what values they hold, or whether a key appears twice. So a ledger that cannot mean anything still passes.

Measured on `origin/main` at bdae608, from `crosscheck/conformance`, with a built binary run on a root whose only file is `conformance/claims.json`. Each of these prints `ERRORS   : 0` and `RESULT: PASS`:

- `{"version":7,"narrative_claims":[]}`. `version` can hold any number.
- `{"version":1,"narrative_claims":[{"status":"reviewed-accurate"}]}`. A claim with no `id`, no `claim`, no `reality` and no `check` loads.
- `{"version":1,"narrative_claims":[{"id":"C","status":"reviewed-accurate","check":{"type":"present_artfact","path":"nope.md"}}]}`. A misspelt `check.type` runs no auto-check, so a missing `nope.md` goes unseen.
- `{"version":1,"narrative_claims":[{"id":"C","status":"reviewed-accurate","check":{"bogus":1},"check":{"type":"manual"}}]}`. The schema check reads only the last copy of `check`, so the unknown key `bogus` is dropped.
- `{"version":1,"narrative_claims":[{"id":"C","status":"reviewed-accurate","tracked_in":null}]}`. `json.Unmarshal` decodes `null` into a string field as `""`, so a `null` value passes as an empty one.

A duplicated `narrative_claims` array is worse. `json.Unmarshal` merges the second array into the first by index, so a field the second array leaves unset keeps the first array's value under the second claim's `id`.

## Proposed outcome
The required fields are decided against the real ledger. Each of its seven claims carries every claim key, and each `check` carries exactly the keys its type uses. The rule is that a key is required when a check or the report reads it, and optional when an absent key and its default mean the same thing.

- **Top level.** `version` is required and must be the JSON number `1`, written `1`. It is the only version that exists. `narrative_claims` is required, as before. `description` is optional and must be a string.
- **Claim.** `id`, `source`, `claim` and `reality` are required and must be strings that are not blank. `status` is required and must be a string. Its value stays with the status allowlist, which already names the claim in its message. `check` is required and must be an object. `tracked_in` is optional and must be a string. An absent `tracked_in` and `""` mean the same thing, and the `known-gap` rule already demands a link where one is needed.
- **Check.** `type` is required and must be `manual` or `present_artifact`. A `manual` check takes no other key, so `{"type":"manual","path":"x"}` fails rather than reading as a check that runs. A `present_artifact` check requires `path`, a string that is not blank, and takes an optional `expect_present`, which must be `true` or `false` and defaults to `true`.
- **No `null`.** No key may hold `null`. A `null` is not a string, a boolean, an object or an array.
- **No duplicate keys.** A key that appears twice in one object fails, at every level, before any other check reads that object. Rejecting outright is the only choice that keeps every copy visible to the schema check. Keeping the first or the last copy would still drop a key unreported.
- A failure is an LL-3 error, `[ledger] cannot parse conformance/claims.json: <reason>`, and the run prints `RESULT: FAIL` and exits 1, as PB-1.25's schema errors do. The reason names the path of the value, for example `narrative_claims[0].check.type is "present_artfact", want one of manual|present_artifact`.
- The real `claims.json` still loads its seven claims and the real tree still passes.
- The spec, `crosscheck/conformance/README.md` and the header comment of `main.go` each say once what is still not yet reached.

## Affected users and systems
- Anyone who edits `claims.json`. A claim with a missing field, a `null`, an unknown check type or a duplicated key now fails the `conformance` job.
- `crosscheck/conformance/main.go`, `main_test.go` and `README.md`. The spec `intent/2026-10-07-ledger-load-fails-closed-spec.md`. `docs/TASKS.md`.

## Constraints
- No protected surface changes. `crosscheck/conformance/` is not in `.claude/rules/protected-surfaces.md`.
- The LL-2 and LL-3 messages keep their prefixes, and PB-1.25's schema messages keep their wording.
- A missing `status` moves from the allowlist error to a schema error. An empty or unknown `status` keeps the allowlist error.
- The rule numbers of the spec stay as they are. LL-9 grows to cover the whole schema, so this change adds no number that an open pull request might also take.

## Open questions
None. The task row asks for a decision on duplicate keys and on the required fields. Both are settled above against the real ledger.

Not yet reached: what the text fields say. Two claims may share an `id`, `source` need not name a real file, `tracked_in` need not name a real issue, and `check.path` may point outside the plugin root. The property that blocks them is a check of each field against the tree and the tracker, and the open question is which of them can be checked without a network call. Unique claim IDs are queued as PB-1.42.
