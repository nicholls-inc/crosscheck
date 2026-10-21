# Intent: `byfuglien` routes a Python data-model change to `cgv_check`

Task: ER-1.21. Governing roadmap item: ER-1.

## Problem statement
`cgv_check` (ER-1.20) runs CGV on a committed Python project and returns CGV's findings JSON and CGV's evidence record. No agent calls it. `byfuglien`, the agent that routes implementation-chain work, has no route to CGV: its Task Classification table sends a change to a Django model or a pydantic model to `/lightweight-verify` ("CRUD / IO-heavy") or to `/reason`, which give tested or LLM-judged evidence. So a change that CGV can check with a checker proved sound over its graph gets weaker evidence, and no CGV record ships with it.

ER-1's acceptance asks that CGV and one Crosscheck pipeline emit a record. CGV emits one, but nothing in Crosscheck carries it into a review.

## Proposed outcome
- `crosscheck/agents/byfuglien.md` gains a route. When a change touches a Python model, data class or pydantic definition, or Python code that may write to one, `byfuglien` calls `cgv_check` on the committed project, with the pull request's base as `baseline`.
- `byfuglien` attaches CGV's evidence record to its report verbatim, and presents the result with CGV's strength and trusted base: proved relative to the translated graph, with extraction and translation trusted, not proved. It names what exit 0 does not cover, and never calls the result "verified".
- A refusal of `cgv_check`, missing CGV binaries, or a missing tool is reported as "CGV: not run", with the tool's errors. It is never a pass, and `byfuglien` does not put its own judgment in place of the run.
- The spec, `intent/2026-10-21-byfuglien-cgv-routing-spec.md`, states the trigger, the call, the report and the quality gate.
- `docs/TASKS.md` marks ER-1.21 `done` with this file as its record.

## Affected users and systems
- A user or reviewer who asks `byfuglien` about a Python change that touches a data model, and the review skills that dispatch `byfuglien` on a pull request (`crosscheck-pr-review`, `crosscheck-verify`, `pr-swarm`'s byfuglien lens). They get CGV's findings and record, or an explicit "not run".
- One Class A protected-surface file, `crosscheck/agents/byfuglien.md`, plus `docs/TASKS.md`, `crosscheck/JOURNAL.md` and a governance note. The change is Tier 3.
- `cgv_check`, CGV and the other agents do not change.

## Constraints
- No claim stronger than CGV's record. The report states the strength and trusted base the record names, and the exit-0 qualifiers of `cgv/README.md` ("What exit 0 promises").
- Exit 0 under a baseline says only that no error is new relative to the base. `runChecker_sound_all` does not cover it (BL-1), and the record comes from the run without a baseline (ER-1.20 decision 1). The report keeps the two apart.
- No guarantee rests on the judgment of an LLM (`docs/VISION.md`). `byfuglien` decides only whether to call the tool. CGV's exit code and record are the evidence.
- No new step, tool or file outside the agent's report. The agent's existing routes stay as they are.
- Only the ER-1.21 row of `docs/TASKS.md` changes.

## Open questions
None remain open. The ones below were decided without the maintainer.

## Decided without the maintainer
The maintainer granted full autonomy for this run in chat on 2026-10-21 and is away. The coordinating agent's chat authorization is recorded in this file, not constituted by it: the grant was given in chat, and this file only records it. Each decision takes the most conservative option: route when in doubt, refuse to guess a revision or a result, and never report a claim stronger than CGV's own record.

1. **The trigger is broad.** `byfuglien` routes when the diff touches a declaration of a Django model, data class, attrs class, pydantic model, `NamedTuple` or `TypedDict`, or touches any `.py` file of a project that declares one. Which functions write to a model is what CGV's extractor finds. An LLM reading the diff would guess it, and a wrong guess would skip the check. A narrower trigger is a later change.
2. **The baseline is the merge base of HEAD and the pull request's base branch**, read with `git merge-base HEAD origin/<base>`, the same commit GitHub diffs the pull request against. The tip of the base branch would count errors fixed on the base after the branch was cut as new in the pull request.
3. **With no pull request, there is no baseline.** When `byfuglien` cannot read a base branch, it calls `cgv_check` without `baseline` and says so in the report. It does not guess `main` or `HEAD~1`.
4. **The project path is the nearest Python project root.** It is the deepest directory that holds every changed `.py` file and a `manage.py`, `pyproject.toml`, `setup.py` or `setup.cfg`, or `.` when there is none. A narrower path can leave out the model a function writes to.
5. **The record goes in the report, not in the repository.** `byfuglien` pastes the record in its report as a JSON block, unchanged. ER-1.20 left where a CGV record lives in a pull request to this task. Committing records is a new artefact with its own review rules, and nothing reads one from the repository yet.
6. **`byfuglien` does not make the work tree clean to run the tool.** `cgv_check` refuses a work tree that differs from HEAD. `byfuglien` does not commit, stash or reset the user's work to get past that, and reports "not run" with the refusal.
7. **The route runs beside the rest of the classification.** It adds a CGV call to whatever skill the request also routes to. It replaces none of them, so `/lightweight-verify` and `/reason` still run when the request asks for them.
8. **Conformance needs no change.** `crosscheck/conformance` checks that each backtick `/<skill>` an agent routes to exists. `cgv_check` is a tool, not a skill, and is written without a leading slash, so the routing check is unaffected and no claim in `claims.json` changes.
