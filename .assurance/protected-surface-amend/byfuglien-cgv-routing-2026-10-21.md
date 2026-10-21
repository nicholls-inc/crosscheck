> **Action needed: Resolve markers and checklist before merging**
> You are being asked to verify this governance-note block as PR reviewer because the edit touches a protected surface. Approving means the protected-surface edit merges as governed and traceable; declining means the author must resolve every unresolved marker and Review Checklist item first. Full explanation: https://github.com/nicholls-inc/crosscheck/blob/main/docs/gates/protected-surface-amendment.md.

## Protected-Surface Amendment

**Target file(s):** `crosscheck/agents/byfuglien.md`
**Class:** A (agent and orchestrator definitions)
**Matched rule:** `crosscheck/agents/*.md`
**Date:** 2026-10-21

### Change Description

`crosscheck/agents/byfuglien.md`, four places:

1. Skills section: a new "Contract graph check (CGV-backed)" table names the `cgv_check` MCP tool and what it returns.
2. Task Classification table: a new row, "Python data-model change", routes a change that touches a Django model, data class, attrs, pydantic, `NamedTuple` or `TypedDict` declaration, or any `.py` file of a project that declares one, to `cgv_check` with the pull request's base as baseline, beside the request's other route.
3. Phase 3: a new "CGV check" procedure. It says how to pick `path` and `baseline` (merge base with the pull request's base branch, or none), that a refusal, a missing tool or missing binaries is "CGV: not run" and never a pass, how to read `exitCode` with and without a baseline, that the record is attached verbatim, and the strength, trusted base and exit-0 qualifiers the report must state.
4. Phase 4: a new "For CGV output" quality gate that checks the above before delivery.

No existing route, skill, gate or frontmatter field is removed or changed.

### Rationale

Task ER-1.21. ER-1.20 (#138) added `cgv_check`, and no agent calls it, so a Python data-model change routed through `byfuglien` gets `/lightweight-verify` or `/reason` and no CGV record. ER-1's acceptance asks that CGV and one Crosscheck pipeline each emit an evidence record. The honesty rules come from `docs/VISION.md` (rule 7 strengths, no LLM judgment as evidence) and `cgv/README.md` "What exit 0 promises". Intent: `intent/2026-10-21-byfuglien-cgv-routing.md`. Spec: `intent/2026-10-21-byfuglien-cgv-routing-spec.md`. Plan: `intent/2026-10-21-byfuglien-cgv-routing-plan.md`.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (Next (4–8 weeks) horizon, item ER-1)
- **Title:** Define the evidence record, and emit it from both tools
- **Scope coverage:** ER-1's scope is the record of evidence that ships with a change. Task ER-1.21 in `docs/TASKS.md` names this agent and this route.

### Authority

- **Authoriser:** hnipps, who granted full autonomy for this run in chat on 2026-10-21. That grant authorizes agent work and merges. It does not resolve the judgment items below. The maintainer's merge is the approval.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `crosscheck/agents/byfuglien.md` | `## Skills` | added: "Contract graph check (CGV-backed)" table with `cgv_check` |
| 2 | `crosscheck/agents/byfuglien.md` | `## Task Classification` | added: "Python data-model change" row and a sentence that it runs beside the other route |
| 3 | `crosscheck/agents/byfuglien.md` | `### Phase 3: Execute the Skill` | added: "CGV check" procedure (BC-2 to BC-7) |
| 4 | `crosscheck/agents/byfuglien.md` | `### Phase 4: Validate Output` | added: "For CGV output" gate (BC-8) |

### Test / Coverage Impact

- No invariant, test or eval changes. No eval in `evals/` drives `byfuglien` through an MCP tool, so BC-1 to BC-8 are prose with no automated check; the spec flags it.
- `crosscheck/conformance` checks the agent's frontmatter and its backtick `/<skill>` routes. The new text routes to a tool, written without a slash, so no claim in `claims.json` changes.
- Output a user can see: a review of a Python data-model change now carries CGV's findings and evidence record, or "CGV: not run" with the reason.
- Class A: no eval or prompt-tuning follow-up is queued.

### Review Checklist

Mechanical items.

- [x] Authoriser is a named human, not a bot and not an agent.
  Command: the handle `hnipps` matches none of `*-bot`, `*[bot]*`, `claude-*` or an agent name. Output: no match.
- [x] Governing roadmap item exists.
  Command: `grep -n "ER-1 — Status" docs/assurance/ROADMAP.md`. Output: `111:**ER-1 — Status: In progress.** Serves rules 3 and 7, and the vision's central`.
- [x] Diff plan names every changed protected file.
  Command: `git diff --name-only origin/main...HEAD`, filtered by the globs in `.claude/rules/protected-surfaces.md`. Output: `crosscheck/agents/byfuglien.md` is the only match. `.assurance/protected-surface-amend/byfuglien-cgv-routing-2026-10-21.md`, `crosscheck/JOURNAL.md`, `docs/TASKS.md` and the three `intent/` files are not protected paths.
- [x] This amendment block appears in the PR body.
  Command: `grep -c "## Protected-Surface Amendment"` on the PR body file. Output: `1`.
- [ ] No unresolved human-verification marker is left.
  Command: `grep -c "REQUIRES HUMAN VERIFICATION:"` on this file, minus this item's own line. Output: 6. The maintainer resolves them; agents must not tick them.

Judgment items. A human decides these. The maintainer's full-autonomy grant authorizes agent merges, not ticks (`.claude/rules/protected-surfaces.md`, Amendment pattern, rule 4), so they stay open.

- [ ] REQUIRES HUMAN VERIFICATION: Rationale is anchored to a concrete trigger (not "cleanup" or "robustness").
- [ ] REQUIRES HUMAN VERIFICATION: Governing roadmap item ER-1 actually covers this change.
- [ ] REQUIRES HUMAN VERIFICATION: Diff plan covers every affected line range and stage.
- [ ] REQUIRES HUMAN VERIFICATION: No invariant is being weakened purely to make a failing test pass.
- [ ] REQUIRES HUMAN VERIFICATION: The report wording states CGV's strength, trusted base and exit-0 qualifiers without overclaiming, against `cgv/README.md` "What exit 0 promises" and the `cgv_check` spec (CK-9), and treats a refusal or missing binaries as "not run".
- [ ] REQUIRES HUMAN VERIFICATION: The decisions taken without the maintainer in `intent/2026-10-21-byfuglien-cgv-routing.md` are acceptable: the broad trigger, the merge base as baseline, no baseline without a pull request, the project-root rule for `path`, the record in the report rather than the repository, and no clean-up of the work tree.
