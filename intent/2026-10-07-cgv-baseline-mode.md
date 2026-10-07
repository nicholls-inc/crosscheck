# Intent: Baseline mode reports only the findings that a change introduces

Task: CG-1.5. Governing roadmap item: CG-1. Issue: #7.

## Problem statement
A whole-repository run of `contracts check` on a real codebase reports hundreds of errors and thousands of warnings. On a private Django codebase, a real bug ranked between error #140 and #180 of about 240 (`cgv/docs/evaluation/real-codebase-evaluation-2026-09.md`). Nobody triages a list that long, so the tool goes unused on pull requests. Each run reports every finding in the project, and nothing tells a reviewer which ones the change under review introduced.

In the replay of historical fixes, the findings for a bug disappeared exactly when its fix landed. So findings are stable enough to compare between two commits, and CGV has no way to compare them.

## Proposed outcome
- `contracts check --write-baseline PATH` writes the findings of a run to a baseline file. The run's output and exit code do not change.
- `contracts check --baseline PATH` compares the findings of a run with a baseline file. It prints only the findings that are not in the baseline, and counts the ones that are. It also counts and lists the baseline findings that the run no longer reports (fixed).
- In baseline mode the run exits 1 when any error is not in the baseline, and 0 when every error is. An incomplete run still exits 2.
- The key that matches a finding with a baseline entry does not contain a line number, so an edit above a finding does not make it new.
- `cgv/README.md` says how to run baseline mode on a pull request in CI.

## Affected users and systems
- Anyone who runs `contracts check` in CI or on a pull request.
- `cgv/src/main.rs`, `cgv/src/report.rs`, a new `cgv/src/baseline.rs`, and tests in `cgv/tests/`.
- `cgv/README.md` ("Output", "Exit codes", "What exit 0 promises").
- `docs/TASKS.md` and `JOURNAL.md`.

## Constraints
- Baseline mode never lets a change that introduces a finding exit 0. Every error the checker reports is either matched by a baseline entry or makes the run exit 1. A match needs equal keys, and the key is defined in the spec. A change that reports a finding under a key whose count did not rise is matched, and the spec names that as a known gap.
- Nothing is silently dropped. Every finding that baseline mode does not print is counted in the report, in both formats.
- Without `--baseline` and `--write-baseline`, the JSON output stays byte for byte the checker's output, and the text report and the exit code do not change.
- No change to the extractor's analyses, the Lean checker, its JSON or the proofs. CGV's protected surfaces are untouched.
- Exit 0 in baseline mode does not carry the guarantee of `runChecker_sound_all`. It says only that every error the checker reported was in the baseline. So `--baseline` cannot be combined with `--evidence-record`, whose claim is that guarantee.
- Per `docs/VISION.md`, a case that the key does not reach is "not yet reached", with its blocking property and open question.

## Open questions
None. Issue #7 states the outcome. The choices it leaves open are settled here.

- **How a baseline is made.** The issue offers a `contracts baseline` command or the normal JSON output. The JSON output has no key that survives an edit above a finding, and a second command would duplicate every option of `check`. A `--write-baseline PATH` flag on `check` makes the baseline in the same run, with the same options.
- **The key.** The issue asks for the site "relative to the enclosing function rather than an absolute line". The key uses the text of the site's line instead of an offset. An offset changes when a line is added above the site inside the same function. The text changes only when the site's own line changes. Both are free of absolute line numbers. The text needs the source file, which the extractor has just read.
- **Repeated keys.** Two findings can share a key, for example two identical writes in one function. The comparison counts each key. When the run has more findings with a key than the baseline has, every finding with that key is printed as new, since the report cannot tell which one the change added.
