# Intent: The text report hides missing-guarantee warnings and reports them as coverage for each module

Task: CG-1.4. Governing roadmap item: CG-1. Issue: #9.

## Problem statement
On a private Django codebase, 7,595 of CGV's 8,374 warnings were missing-guarantee warnings, on 6,923 distinct lines. Such a warning (`missingPostconditionWarning` in `cgv/prover/ContractGraph/Diagnostics.lean`) says that a hop's source has no guarantee of the constraint kind that its target requires, so the requirement passes vacuously. It does not say that CGV found a bad value. `contracts check --format text` prints each one as a `WARNING` block next to the errors, worded as "`f` guarantees nullability (unspecified)". Readers take the blocks for findings, and the few errors are lost among them.

The report cannot drop them either. A run with missing-guarantee warnings and no errors exits 0, and the README's "What exit 0 promises" says that such a run has unknown paths, not consistent ones. A text report that hid the warnings without counting them would turn those unknown paths into a silent pass.

## Proposed outcome
- `--format text` no longer prints a block for each missing-guarantee warning by default. `--warnings` prints them, labelled `UNVERIFIED`, with the text "no nullability guarantee for the value `f` passes to `T`".
- The text report prints a coverage section. For each top-level module it gives the number of edges checked and the number of requirements that could not be verified for lack of a source guarantee, by constraint kind. The section says that these hops are not yet reached, names the blocking property (the source has no guarantee of the required kind, so the requirement passes vacuously) and the open question (which guarantee the extractor could infer for such a value, or which annotation it should ask for).
- The `RESULT:` line counts the unverified requirements apart from the errors and the other warnings, whether or not their blocks are printed.
- Other warnings, such as an unresolved dependent bound, print as before. `--no-warnings` still hides them.
- The JSON output, the default format, does not change, so the benchmark and other tools read the same warnings.
- The exit code does not change.

## Affected users and systems
- Anyone who reads `contracts check --format text`.
- `cgv/src/report.rs`, `cgv/src/main.rs`, a reader of the edges table of the contract database, the unit tests in `report.rs` and the end-to-end tests in `cgv/tests/`.
- `cgv/README.md` ("Output" and "What exit 0 promises") and section 5.3 of `cgv/docs/design/system-design.md`.
- `docs/TASKS.md`.

## Constraints
- No change to the extractor's analyses, the Lean checker, its JSON or the proofs. CGV's protected surfaces are untouched.
- Hiding a warning must never hide that a requirement went unverified. Every hidden warning is counted in the coverage section and in the `RESULT:` line. A warning the report cannot classify as a missing guarantee is printed as a `WARNING`, never hidden.
- The edge counts of the coverage section must sum to the checker's `edges_checked`, so the section reports the same graph the checker checked.
- An incomplete run (exit 2) verified nothing, so the report prints no coverage section for it.
- Per `docs/VISION.md`, an unverified hop is "not yet reached", with its blocking property and open question, never "out of scope".

## Open questions
None. Issue #9 states the outcome. Two choices it leaves open are settled here.

- The issue asks for "hops checked" per module. The checker reports `edges_checked`, the edges of the translated graph, and nothing finer. The coverage section counts those edges per module, from the same contract database the checker reads, so its counts sum to `edges_checked`. It counts unverified requirements, not hops, because one hop can leave several kinds unverified.
- A module is the first directory of a file's path relative to the checked path, or the file's name without `.py` for a file at the top. A warning belongs to the module of its write or call site, and an edge to the module of its site, or of its source node when it has no site.
