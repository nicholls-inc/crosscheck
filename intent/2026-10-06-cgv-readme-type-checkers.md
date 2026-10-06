# Intent: The CGV README says what CGV is for next to type checkers, and what exit 0 promises

Task: CG-1.1. Governing roadmap item: CG-1. Issue: #10.

## Problem statement
`cgv/README.md` does not say how CGV relates to mypy or pyright. A reader cannot tell which bugs CGV finds that a type checker misses, or the reverse. Its exit-code table says exit 0 means "Every data path (function → model node) consistent". Without qualifiers, a reader can take that as a claim about every write in the program. The proof (`runChecker_sound_all`) covers only the paths in the translated graph. Writes that the extractor does not see are not in that graph. A missing guarantee gives a warning and still exits 0.

Issue #10 reports a comparison with mypy and pyright on the fixtures. Its results and tool versions are not in the repository, so no one can rerun them.

## Proposed outcome
- The README has a "When to use it" section. It names the constraints CGV checks that a type checker does not (length, precision, range, choices across several hops), and the bugs a type checker reaches that CGV does not.
- The README has a table that compares CGV, mypy and pyright on the cases of `cgv/bench/corpus`, with the tool versions. A script in `cgv/scripts/` produces the table, and a pinned requirements file fixes the versions, so anyone can rerun it.
- The exit-code table and a "What exit 0 promises" section state the qualifiers: only paths the extractor discovered; writes made by framework code, raw SQL and fixtures are not yet reached; a missing guarantee is a warning.

## Affected users and systems
- Anyone deciding whether to run CGV on a Python codebase, and anyone reading an exit 0.
- `cgv/README.md`, a new `cgv/scripts/typecheck_compare.py` with tests in `cgv/scripts/tests/`, and a new `cgv/bench/typecheckers/requirements.txt`.
- `docs/TASKS.md`.

## Constraints
- No change to the extractor, the checker or the proofs.
- Every claim in the new text either cites a rerunnable command or a pinned external source.
- Per `docs/VISION.md`, a class of code the tool does not reach is "not yet reached", with the blocking property and the open question.
- The comparison script needs no network once the pinned packages are installed and pyright has fetched its Node package, and it does not run in CGV CI, which installs no Python type checkers.

## Open questions
None. The issue states the outcome.

The issue's other claims (mypy catches Optionals through `@property`, tuple unpacking and comprehensions, and wrong numeric base types) are not in the bench corpus, so this change does not state them. Adding bench cases for them is CG-1 work that the README can cite once the cases exist.
