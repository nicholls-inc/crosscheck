# Spec: `byfuglien` routes a Python data-model change to `cgv_check`

Intent: `intent/2026-10-21-byfuglien-cgv-routing.md`. Governing roadmap item: ER-1. Task: ER-1.21.

The tool is `cgv_check`, specified in `intent/2026-10-21-cgv-check-mcp-tool-spec.md` (CK-1 to CK-13). This spec changes only `crosscheck/agents/byfuglien.md`.

## Requirements

- **BC-1. Trigger.** The Task Classification table has a row for a Python data-model change. It matches when the change touches a declaration of a Django model, a data class, an attrs class, a pydantic model, a `NamedTuple` or a `TypedDict`, or touches any `.py` file of a project that declares one. The path is `cgv_check` with the pull request's base as baseline. The row runs beside the request's other route and replaces none.
- **BC-2. The call.** `byfuglien` calls `cgv_check` with:
  - `repoPath`: the absolute path of the work tree;
  - `path`: the deepest directory that holds every changed `.py` file and a `manage.py`, `pyproject.toml`, `setup.py` or `setup.cfg`, relative to the top level, or `.` when there is none;
  - `baseline`: the output of `git merge-base HEAD origin/<base>`, where `<base>` is the pull request's base branch. When no base branch can be read, `baseline` is left out, and the report says that the findings are not against a base.
  `byfuglien` does not commit, stash or reset the work tree to make the call succeed.
- **BC-3. Not run.** When `cgv_check` returns `success: false`, when the tool is not available, or when the call fails, the report says "CGV: not run" and quotes the tool's errors. A missing-binaries error carries the build command that `cgv_check` prints, and the report keeps it. "Not run" is never a pass, and `byfuglien` does not replace the run with its own reading of the code.
- **BC-4. The result.** On `success: true` the report gives `commit`, `baseline`, `exitCode` and the errors and warnings in `findings`, and reads `exitCode` as follows:
  - with a baseline, exit 1 means at least one error is new relative to the base, and exit 0 means no error is new. Exit 0 under a baseline is not covered by `runChecker_sound_all`, and the report says so. A change that removes one finding and adds an equal one with the same key is matched and not reported, which `cgv/README.md` marks not yet reached, and the report says so too;
  - without a baseline, exit 1 means CGV found at least one error;
  - a warning marks a requirement CGV could not show: an unknown path, not a consistent one.
- **BC-5. The record.** When `record` is not `null`, the report attaches it verbatim as a JSON block and states its claim as CGV's record does: strength `proved`, relative to the translated graph and the constraints the extractor derived at that commit, with the trusted base the record lists. The extractor and `Translation.lean` are not proved, and `BehaviorModel.lean` is trusted, not proved. When `record` is `null`, the report says that no record was written because an error remains at HEAD, even when the baseline run exits 0.
- **BC-6. What exit 0 does not cover.** Every report with a CGV result names the qualifiers of `cgv/README.md` "What exit 0 promises": only the paths the extractor discovered; writes outside the project's code (Django or DRF writes on the project's behalf, raw SQL, fixtures) are not yet reached; only paths that end at a model node; relative to the translated constraints, not to Django or pydantic, and for data class, attrs, `NamedTuple` and `TypedDict` fields only for a type-correct program; a warning can rest on the project's callers; a missing guarantee is a warning, not an error, and a record next to warnings does not cover the paths the warnings name.
- **BC-7. Words.** The report never calls a CGV result "verified", "safe" or a "pass", and never states a strength the record does not carry.
- **BC-8. Quality gate.** Phase 4 has a CGV gate that checks BC-3 to BC-7 before delivery.

## Concerns flagged, not resolved here

- **No check enforces BC-1 to BC-8.** They are prose in an agent file. An eval that runs `byfuglien` on a Python data-model change and checks its report is not yet reached; the property that blocks it is that no eval in `evals/` drives an agent through an MCP tool, and the open question is how to run CGV in an eval without the binaries in CI (ER-1.22).
- **The trigger is decided by an LLM reading the diff.** BC-1 is broad so that a miss is unlikely, but whether the route fires still rests on `byfuglien`'s reading. A deterministic trigger, for example a script that lists the declarations a diff touches, is not yet reached.
- **`cgv_check`'s own gaps carry over.** CGV runs with its default options, on the host, and the CK-5 known gap on line-ending conversions applies (ER-1.23). The report states CGV's result, not more.
- **The rerun command names local paths.** The record's rerun names the checker by its absolute path on the machine that ran it, as the `cgv_check` spec flags.
