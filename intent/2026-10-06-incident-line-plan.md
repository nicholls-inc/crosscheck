# Plan: Count only a whole `Fixes-Incident:` line as an incident reference

Intent: `intent/2026-10-06-incident-line.md`
Spec: `intent/2026-10-06-incident-line-spec.md` (IE-9, IE-5 and IE-6 revised)
Governing roadmap item: PB-1. Task: PB-1.16. Tier: 3.

## Order of work

1. Write the governance note `.assurance/protected-surface-amend/incident-line-2026-10-06.md`, naming the protected files below.
2. Add the IE-9 cases to `scripts/ci/incident-eval-check.test.mjs` and flip the "body as one text" case. Run `node --test scripts/ci/incident-eval-check.test.mjs` and see the new cases fail against the current script.
3. In `scripts/ci/incident-eval-check.mjs`, replace the pattern in `findIncidentId` with the IE-9 line pattern and split the body into lines.
4. Run `node --test scripts/ci/*.test.mjs`.
5. Replay #62: in a fresh clone of `origin`, run the script with #62's body, number, base and head. It prints the skip line and exits 0. Run it the same way for #61, which passed, to check nothing else changed.
6. Update the trigger wording in stage 5 of `docs/assurance/DEVELOPMENT-FRAMEWORK.md` and in `docs/gates/tier-layer-gate.md`, add a note to the 2026-09-30 spec pointing here, add a `JOURNAL.md` entry, and set PB-1.16 to `done` in `docs/TASKS.md`.

## Files

| File | Protected | Change |
|---|---|---|
| `scripts/ci/incident-eval-check.mjs` | yes (`scripts/ci/**`) | IE-9 line pattern, body split into lines |
| `scripts/ci/incident-eval-check.test.mjs` | yes (`scripts/ci/**`) | IE-6 revised cases |
| `docs/assurance/DEVELOPMENT-FRAMEWORK.md` | yes (`docs/assurance/**`) | stage 5 trigger wording |
| `docs/gates/tier-layer-gate.md` | no | trigger wording |
| `intent/2026-09-30-incident-eval-range-spec.md` | no | pointer to the revision |
| `docs/TASKS.md`, `JOURNAL.md` | no | PB-1.16 `done`, journal entry |
| `.assurance/protected-surface-amend/incident-line-2026-10-06.md` | no | governance note |
| `intent/2026-10-06-incident-line*.md` | no | stage artefacts |

## Risks

- **A real incident reference written as a sentence is skipped.** The check exits 0 where it used to exit 1. The `incident` label still forces the check, and the docs say to write one whole line per incident. The spec records this under "Not yet reached".
- **Merge conflict with #66.** Both change the script, its tests and stage 5. The conflicting lines are small: `findIncidentId`, one test, and one sentence of stage 5. #66 renames the test helper `squashMergedPr` to `mergedPr`, so the second pull request to merge renames the helper in the new cases.
- **The fix is proved on a live run only after merge.** Step 5 replays #62 against the real remote before then.

## Proof that it worked

- `node --test scripts/ci/*.test.mjs` passes, including the IE-6 revised cases, which fail on `origin/main`.
- Step 5 prints `no incident reference — skipped` for #62 and #61.
- After merge, this pull request's own Incident Eval Check run succeeds, though its body and commits describe the trigger.
