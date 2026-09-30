# Plan: Anchor the tier gate's `Tier:` line, and label unchecked code "not yet reached"

Intent: `intent/2026-09-30-tier-anchor.md`
Spec: `intent/2026-09-30-tier-anchor-spec.md` (TG-1 and TG-8 revised, TG-11, DOC-6)
Governing roadmap item: PB-1. Task: PB-1.4. Tier: 3.

This plan is not the root `plan.md`, which belongs to an earlier change. The pull request body cites this file with a `Plan:` line.

## Order of work

1. Write the governance note `.assurance/protected-surface-amend/tier-anchor-2026-09-30.md`, naming the three protected files below.
2. Add the TG-11 cases to `scripts/ci/tier-gate.test.mjs`, and change the two existing TG-8 cases to the new line format. Run `node --test scripts/ci/*.test.mjs` and see the new cases fail against the current gate.
3. In `scripts/ci/tier-gate.mjs`:
   - read the tier with the citation prefix, `^[ \t]*(?:[-*+>][ \t]+)?Tier:[ \t]*([123])\b`, multiline and case-insensitive (TG-1);
   - replace `EVIDENCE_CLASSES` with a table of rows, each with a path test and one of the three kinds from TG-8, and render each kind's line from the row. The report groups files by row, not by label text (TG-8).
4. Run `node --test scripts/ci/*.test.mjs`.
5. Run the gate against every tracked file (`git ls-files`) and read the report: no non-prose path says `none required at this tier`.
6. Update the documents in DOC-6.
7. Set PB-1.4 to `done` in `docs/TASKS.md`, with this intent as its record, and add a root `JOURNAL.md` entry.

## Files

| File | Protected | Change |
|---|---|---|
| `scripts/ci/tier-gate.mjs` | yes (`scripts/ci/**`) | TG-1 anchor, TG-8 table |
| `scripts/ci/tier-gate.test.mjs` | yes (`scripts/ci/**`) | TG-11 cases |
| `docs/assurance/TIER-LAYER-MAP.md` | yes (`docs/assurance/**`) | declaration rule and evidence table (DOC-6) |
| `docs/gates/tier-layer-gate.md` | no | declaration rule and pass report (DOC-6) |
| `intent/2026-09-29-deterministic-evidence-spec.md` | no | pointers from TG-1 and TG-8 to this spec |
| `docs/TASKS.md` | no | PB-1.4 `done` |
| `JOURNAL.md` | no | entry |
| `.assurance/protected-surface-amend/tier-anchor-2026-09-30.md` | no | new governance note |
| `intent/2026-09-30-tier-anchor*.md` | no | stage artefacts |

## Risks

- **A PR body that declared its tier mid-line.** It now declares nothing, so the gate asks for a declaration, or uses the label or the protected floor. The last four merged PR bodies (#52 to #55) put `Tier:` at the start of a line.
- **Long report lines.** A not-yet-reached line carries two clauses. The report is read by people, and one line per class keeps it short.
- **Row 9 is broad.** It labels configuration as unchecked code. That is accurate: no workflow checks it. A new workflow that checks a path needs a new checked row, or the report understates the evidence. That was already true of the old table.

## Proof that it worked

- `node --test scripts/ci/*.test.mjs` passes, including every TG-11 case, and the new cases fail against the old gate.
- Step 5 lists no non-prose path under `none required at this tier`.
- The gate, run locally with this branch's changed files and the PR body, passes at Tier 3.
