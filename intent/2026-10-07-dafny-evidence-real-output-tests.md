# Intent: Test `dafny_evidence` against real Dafny output in `npm test`

Task: ER-1.9. Governing roadmap item: ER-1. Spec: `intent/2026-10-06-dafny-evidence-record-spec.md` (the Tests section and DE-9 amended).

## Problem statement
`npm test` covers DE-5 to DE-8 of `dafny_evidence` only with Dafny output written by hand. The integration test's verification log, audit text and version line are strings a person typed, so they show that the tool parses what the test author thought Dafny prints, not what Dafny 4.11.0 prints. Only `npm run test:e2e` runs the real image, and `npm test` skips that suite, so CI never sees real Dafny output.

No test runs `scripts/check-evidence-record.mjs`, ER-1.4's checker, on a record that `dafny_evidence` writes. DE-10 says the record satisfies EV-1 to EV-12, and nothing checks that claim with the checker that defines those rules.

## Proposed outcome
- The output of real Dafny 4.11.0 runs is committed as fixtures: for each of eight Dafny programs, the arguments, exit code, standard output and standard error of every Dafny run `dafny_evidence` makes on it, the image ID, and the program's source files. The programs cover a proved top-level lemma, a lemma in a class in a module, a proved include, an include whose lemma fails, an included `{:axiom}`, a top-level `{:axiom}`, an `assume`, and a failing postcondition.
- A new integration test, run by `npm test`, builds a real git repository from each fixture's sources, replays the recorded runs in place of Docker, and compares what `dafny_evidence` returns with a literal. The replay refuses a run whose arguments differ from the recorded ones, so a change to how the tool calls Dafny fails the test until the fixtures are recorded again.
- The same test writes the proved program's record to a file and runs `node scripts/check-evidence-record.mjs` on it, which must exit 0.
- An end-to-end test records the fixtures from the real image when `RECORD_DAFNY_FIXTURES=1` is set, and otherwise compares the live output with the committed fixtures, so a new image that changes Dafny's output fails `npm run test:e2e`.
- `docs/TASKS.md` marks ER-1.9 `done` with this file as its record.

## Affected users and systems
- Maintainers of `crosscheck/mcp-server` get a failing `npm test` when a change to the tool no longer accepts or refuses what real Dafny output says it should.
- `dafny_evidence` does not change. No caller sees a difference.

## Constraints
- `npm test` must not need Docker, since CI runs it without the Dafny image.
- The recorded output is Dafny's own: the fixtures drop only Docker's platform-mismatch warning, a line the Docker CLI prints on an arm64 host, so they read the same whichever host recorded them. The live comparison ignores the duration column of the verification log, which changes on every run.
- No new dependency.

## Open questions
None that block the work. The fixtures pin Dafny 4.11.0. A new Dafny release needs the fixtures recorded again, and the end-to-end comparison says so when it fails.
