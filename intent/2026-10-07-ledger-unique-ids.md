# Intent: two claims in `claims.json` cannot share an `id`

Task: PB-1.42. Governing roadmap item: PB-1. Found in PB-1.40 (`intent/2026-10-07-ledger-required-fields.md`, open questions).

## Problem statement
PB-1.40 made `loadLedger` in `crosscheck/conformance/main.go` require a non-blank `id` on every claim in `claims.json`. It does not require the `id` to be unique. Every ledger message names a claim by its `id` alone (`claim %s is UNREVIEWED`, `claim %s auto-check failed`, `claim %s has unknown status`), so when two claims share one, a reader of the CI log cannot tell which claim failed.

Measured on `origin/main` at 24bc384, from `crosscheck/conformance`, with `go run .` on a root whose only file is `conformance/claims.json` holding two claims with `"id":"C1"`, one `reviewed-accurate` and one `unreviewed`. It prints one error, `[ledger] claim C1 is UNREVIEWED — triage required`, and the `NARRATIVE LEDGER` section lists both claims as `C1`. Nothing in the output says which of the two claims the error means.

## Proposed outcome
- Two claims whose `id`s are equal after trimming surrounding white space and Unicode case folding fail. The comparison is `strings.EqualFold(strings.TrimSpace(a), strings.TrimSpace(b))`.
- The fault is an LL-3 error, as PB-1.40's schema errors are: `[ledger] cannot parse conformance/claims.json: narrative_claims[<j>].id "<b>" repeats narrative_claims[<i>].id "<a>"`, where `<i>` is the earlier claim. The `id`s are quoted, so white space that tells them apart is visible. The run prints `RESULT: FAIL`, exits 1, and the ledger is empty.
- The real `claims.json` keeps its seven claims, whose seven `id`s are distinct, and the real tree still passes.

### Why fold case and trim, rather than compare exactly
The `id` exists so a person can tell claims apart in a log line. `C1` and `C1 ` print the same in every ledger message, because those messages do not quote the `id`. `CLAIM-LAYERS` and `claim-layers` read as the same claim to a person searching the log or the ledger. Comparing exactly would let both pairs through. Folding and trimming rejects strictly more ledgers than exact comparison, and the real ledger passes either way, so the stricter rule costs nothing today.

White space inside an `id` is not trimmed, so `C 1` and `C1` stay distinct. They print differently.

## Affected users and systems
- Anyone who edits `claims.json`. A claim that reuses an earlier claim's `id`, up to case and surrounding white space, now fails the `conformance` job.
- `crosscheck/conformance/main.go`, `main_test.go` and `README.md`. The spec `intent/2026-10-07-ledger-load-fails-closed-spec.md`. `docs/TASKS.md`.

## Constraints
- No protected surface changes. `crosscheck/conformance/` is not in `.claude/rules/protected-surfaces.md`.
- The rule numbers of the spec stay as they are. LL-9 grows by one bullet, so this change adds no number that an open pull request (#119 for PB-1.41) might also take.
- The check runs after each claim's own shape checks, so a claim with a missing or blank `id` still reports that fault first.

## Open questions
None. The task row asks for the comparison to be decided. It is settled above.

Not yet reached: two `id`s that differ only by a character a reader cannot see, such as a zero-width space (`C1` and `C1​`), or by a look-alike letter from another script (Latin `C1` and Cyrillic `С1`), are still distinct. The property that blocks it is a definition of which characters an `id` may hold, and the open question is whether that is an allowlist of characters or a Unicode confusables check. PB-1.43 already queues zero-width characters in `id`.
