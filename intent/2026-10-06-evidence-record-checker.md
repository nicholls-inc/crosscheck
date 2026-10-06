# Intent: A deterministic checker for evidence records

Task: ER-1.4. Governing roadmap item: ER-1. Spec: `intent/2026-10-06-evidence-record-spec.md`.

## Problem statement
`intent/2026-10-06-evidence-record-spec.md` defines version 1 of the evidence record and the rules EV-1 to EV-13 that a checker applies. No checker exists. A record with a claim that names no strength, or no rerun command, is accepted by anything that reads it today, so the roadmap's acceptance for ER-1 is not met. ER-1.2 (CGV) and ER-1.3 (one Crosscheck pipeline, not yet chosen) emit records, and nothing outside each emitter's own tests says whether a record is well formed.

## Proposed outcome
- `scripts/check-evidence-record.mjs` takes the path of a record and applies EV-1 to EV-12 as the spec writes them. It exits 0 on a well-formed record, 1 with one line per broken rule, and 2 when it cannot read the file, the file is not JSON, or it gets no path (EV-13).
- The rules live in a pure function, `checkRecord`, that takes a parsed value and returns the problems. The command line only reads, parses and prints.
- `scripts/check-evidence-record.test.mjs` holds every case the spec's "Tests the checker must have" lists, plus a negative case for each rule EV-1 to EV-12. Run with `node --test scripts/check-evidence-record.test.mjs`.
- `docs/TASKS.md` marks ER-1.4 `done` with this file as its record, and adds a `todo` row to run the checker's tests in CI.

## Affected users and systems
- Whoever reads an evidence record: a reviewer, an auditor, or a CI job, who wants to know the record is well formed before reading its claims.
- ER-1.2 and ER-1.3, whose records the checker can read once they merge.
- `docs/TASKS.md` and `JOURNAL.md`, for the row and the entry of this task.

## Constraints
- EV-13: no network, no LLM, and no command from the record is run. The checker reads one file.
- No new dependency. Node's standard library only, as for `scripts/ci/*.mjs`.
- The spec fixes three details a runtime's defaults get wrong. White space is the Unicode `White_Space` set, so U+0085 is blank and U+FEFF is not, which `String.prototype.trim` reverses. A leading byte order mark makes the file not JSON. An integer is any finite whole double of magnitude at most 2^53, whatever its written form.
- The checker checks shape, not truth. A record whose rerun commands fail still passes. Rerunning claims is not yet reached; the spec's "Concerns flagged" names the blocking property (a pinned, sandboxed environment for each command) and the open question (how to pin the toolchains).
- Duplicate JSON keys are not rejected: `JSON.parse` keeps the last, and the spec does not require a parser of its own.
- No protected surface. The checker sits in `scripts/`, not `scripts/ci/`, so it adds no CI enforcement. Running its tests in CI edits `.github/workflows/**`, a protected surface, and becomes its own row.

## Open questions
None. The spec decides every rule, the output format and the exit codes.
