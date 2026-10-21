# Intent: The MCP server runs CGV on a committed Python project (`cgv_check`)

Task: ER-1.20. Governing roadmap item: ER-1.

## Problem statement
CGV emits an evidence record (`contracts check --evidence-record`, ER-1.2), and Crosscheck emits one for a Dafny run (`dafny_evidence`, ER-1.3). No Crosscheck tool runs CGV. An agent that wants CGV's verdict on a Python change has to find the two CGV binaries, call the command line in a shell, and read the JSON and the record back. ER-1.21 needs `byfuglien` to route a change to a Python model, data class or pydantic definition to CGV with the pull request's base as baseline, and to attach the record to its report. That needs a tool it can call.

CGV's command line refuses `--baseline` together with `--evidence-record` (BL-1 in `intent/2026-10-07-cgv-baseline-mode-spec.md`). Exit 0 under `--baseline` says only that every error had a baseline finding with an equal key, which `ContractGraph.runChecker_sound_all` does not cover. So "run with `--baseline` and `--evidence-record`" cannot be one CGV run.

## Proposed outcome
- A new MCP tool, `cgv_check`, in `crosscheck/mcp-server` beside `dafny_evidence`. It takes a git work tree, the path of the Python project in it, and an optional baseline revision.
- It runs `crosscheck-contracts contracts check` on the project at HEAD with `--evidence-record`. When a baseline revision is given, it also writes a baseline with `--write-baseline` from the project as committed at that revision, and runs `contracts check` at HEAD again with `--baseline`.
- It returns CGV's findings JSON as CGV printed it (the `--baseline` run's when a baseline is given), CGV's exit code for those findings, and CGV's evidence record. The record comes from the run without a baseline, and it is `null` when that run exited 1. The tool never writes or edits a record itself.
- It refuses, as `dafny_evidence` does, when the work tree differs from HEAD, when the project path is not committed, or when HEAD or the work tree changes while CGV runs. It fails with the exact build command when the CGV binaries are missing.
- The spec, `intent/2026-10-21-cgv-check-mcp-tool-spec.md`, states the inputs, the runs, the result and each refusal.
- `CLAUDE.md` and `crosscheck/README.md` list the new tool. `docs/TASKS.md` marks ER-1.20 `done` with this file as its record.

## Affected users and systems
- A caller of the MCP server gets CGV's findings and record for a committed Python project without a shell. No agent calls the tool yet. ER-1.21 wires `byfuglien` to it.
- The MCP server gains its eighth tool. The seven existing tools do not change. The git helpers that `dafny_evidence` uses move to `src/git.ts` so both tools share them, with no change in behaviour.
- CGV does not change. Open PR #133 adds a `counterexample` field to CGV's JSON. The tool passes the JSON through, so that change needs no follow-up here.

## Constraints
- The record must pass `scripts/check-evidence-record.mjs`. CGV builds it, so the tool checks only that it is a version 1 record of HEAD, and the tests run the checker on a real one.
- No claim stronger than CGV's own record. The tool decides nothing about strength. The record is CGV's file, read back byte for byte as JSON.
- CGV runs on the host, not in a container. The extractor parses Python and does not run it, and the checker reads a SQLite file. Sandboxing it as the Dafny runs are is not yet reached; the spec flags it.
- No new dependency.

## Open questions
None remain open. The ones below were decided without the maintainer.

## Decided without the maintainer
The maintainer granted full autonomy for this run on 2026-10-21 and was away. Each decision takes the most conservative option: refuse rather than guess, and never report a claim stronger than CGV's own record.

1. **`--baseline` and `--evidence-record` are two runs, not one.** CGV forbids the pair on purpose (BL-1). The record always comes from a run without a baseline, so a pre-existing error means no record, even when the change adds no error. The baseline run only decides which findings are returned. The alternative, a record built from the baseline run, would claim more than CGV does.
2. **The baseline is a git revision, not a file.** ER-1.21 has the pull request's base, a revision. The tool writes the baseline itself with the same binaries and the same options, which closes the known gap that a baseline made with other options or another CGV version mismatches (BL known gaps). The project is exported at that revision with `git archive`, whose `.gitattributes` `export-ignore` and `export-subst` can only remove or change baseline findings, so they can only make more findings new.
3. **The whole work tree must match HEAD**, as `dafny_evidence` requires (DE-3), not only the checked path, as CGV requires (CR-3). It is the stricter rule and the one the task names.
4. **The binaries come from `CGV_CONTRACTS` and `CGV_CHECKER`, or else from `PATH`.** A variable that is set but names no executable file is refused, not passed over for `PATH`. The checker is always passed with `--lean-checker`, so the record's rerun command names it.
5. **The inputs are the work tree, the project path and the baseline revision only.** `--overrides`, `--exclude`, `--django-version` and `--max-states` are not passed, so CGV's defaults apply, and `--allow-parse-errors` is never passed, because the record does not name the files it skips (CR spec concern). Passing the others through is not yet reached. A caller that needs them runs CGV directly.
6. **The record is returned, not written into the repository.** `dafny_evidence` takes an `outputPath`. ER-1.21 attaches the record to a report, so this tool does not take one yet.
7. **A project path that does not exist at the baseline revision is refused.** The alternative, an empty baseline that makes every finding new, is a guess about what the caller meant.
