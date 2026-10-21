> **Action needed: Resolve markers and checklist before merging**
> You are being asked to verify this governance-note block as PR reviewer because the edit touches a protected surface. Approving means the protected-surface edit merges as governed and traceable; declining means the author must resolve every unresolved marker and Review Checklist item first. Full explanation: https://github.com/nicholls-inc/crosscheck/blob/main/docs/gates/protected-surface-amendment.md.

## Protected-Surface Amendment

**Target file(s):** `crosscheck/skills/assurance-layer-audit/SKILL.md`
**Class:** A (skill behaviour definitions)
**Matched rule:** `crosscheck/skills/*/SKILL.md`
**Date:** 2026-10-21

### Change Description

`crosscheck/skills/assurance-layer-audit/SKILL.md`, three places:

1. Step 4, Layer 3: the "Python / TypeScript" bullet, which said runtime contract libraries give dynamic checks and "not formal" subgraph verification, becomes two bullets. The Python bullet names CGV (`cgv/` in the Crosscheck repository, run with `contracts check`) as Layer 3 reach for data paths into the declared fields of Django models, data classes, attrs, pydantic models, `NamedTuple` and `TypedDict`, with its strength (Lean checker deterministic and proved sound over the graph it is given), its trusted base (`Translation.lean`, `BehaviorModel.lean`; extractor untrusted but auditable) and the exit-0 qualifiers of `cgv/README.md`. Other Python code stays not yet reached, with the blocking property and the open question. The TypeScript bullet says not yet reached, with the blocking property and the open question that `crosscheck/docs/assurance-hierarchy.md` gives for Layer 3.
2. Instructions paragraph: "a Python repo loses most of Layer 1" becomes "Layer 1 reaches only pure, sequential code written in Dafny and extracted to Python or Go", and "almost nobody has Layer 3 end-to-end subgraph verification today" gains "outside CGV's Python data paths".
3. Verification Checklist: the Layer 1 line drops Python, and one line is added for CGV's Layer 3 reach in a Python repository.

### Rationale

Task VA-1.19. The skill told a Python repository that Layer 3 has only runtime contract libraries, while `crosscheck/docs/assurance-hierarchy.md` (Layer 3 row) records that CGV checks Python data paths with a checker proved sound over its graph. VA-1's acceptance asks that each class a tool does not reach be described as "not yet reached", with the blocking property and the open question, and that what a tool does reach be stated with its strength. The Layer 1 lines contradicted Step 4 of the same skill and `/extract-code`, which compiles verified Dafny to Python. Intent: `intent/2026-10-21-cgv-layer-3-python.md`. Spec: `intent/2026-10-21-cgv-layer-3-python-spec.md`. Plan: `intent/2026-10-21-cgv-layer-3-python-plan.md`.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (Next (4-8 weeks) horizon, item VA-1)
- **Title:** Bring what Crosscheck tells its users in line with the vision
- **Scope coverage:** VA-1's scope covers the skills that "still describe the positions that the vision replaces". Task VA-1.19 in `docs/TASKS.md` names this skill and the Layer 3 claim.

### Authority

- **Authoriser:** hnipps, who granted full autonomy for this run in chat on 2026-10-21. The merge is the approval.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `crosscheck/skills/assurance-layer-audit/SKILL.md` | Step 4, Layer 3, Python / TypeScript bullet | replaced: one bullet becomes a Python bullet (CGV reach, strength, trusted base, exit-0 qualifiers, not-yet-reached remainder) and a TypeScript bullet (not yet reached) |
| 2 | `crosscheck/skills/assurance-layer-audit/SKILL.md` | Instructions paragraph | reworded: Layer 1 and Layer 3 limits |
| 3 | `crosscheck/skills/assurance-layer-audit/SKILL.md` | Verification Checklist | reworded: Layer 1 line drops Python; added: CGV line |

### Test / Coverage Impact

- No invariant, test or eval changes. No step, output section, JSON field or threshold is added or removed, so `go run . ..` in `crosscheck/conformance` is unaffected.
- Output a user can see: a Python repository's Layer 3 projection names CGV when it declares such fields, with the trusted base and exit-0 qualifiers. A TypeScript repository's Layer 3 reads "not yet reached". A Python repository's report no longer says Layer 1 is unreachable.
- Class A: no eval or prompt-tuning follow-up is queued. No eval exercises `/assurance-layer-audit`.
- `/assurance-init` and `/assurance-status` make no Layer 3 claim, so they are not edited; the intent records why.

### Review Checklist

Mechanical items.

- [x] Authoriser is a named human, not a bot and not an agent.
  Command: the handle `hnipps` matches none of `*-bot`, `*[bot]*`, `claude-*` or an agent name. Output: no match.
- [x] Governing roadmap item exists.
  Command: `grep -n "VA-1 — Status" docs/assurance/ROADMAP.md`. Output: `101:**VA-1 — Status: In progress.** Serves rule 1 and the scope section of the`.
- [x] Diff plan names every changed protected file.
  Command: `git diff --name-only origin/main...HEAD`, filtered by the globs in `.claude/rules/protected-surfaces.md`. Output: `.assurance/protected-surface-amend/cgv-layer-3-python-2026-10-21.md` is not a protected path; `crosscheck/skills/assurance-layer-audit/SKILL.md` is the only match.
- [x] This amendment block appears in the PR body.
  Command: the PR body contains the `## Protected-Surface Amendment` heading. Output: it does.
- [x] No unresolved human-verification marker is left.
  Command: count the markers in this file. Output: 0.

Judgment items. A human decides these, or an agent ticks them on the maintainer's explicit instruction.

- [ ] Rationale is anchored to a concrete trigger (not "cleanup" or "robustness").
- [ ] Governing roadmap item actually covers this change.
- [ ] Diff plan covers every affected line range and invariant ID / stage.
- [ ] No invariant is being weakened purely to make a failing test pass.
- [ ] CGV's reach, strength, trusted base and exit-0 qualifiers are stated without overclaiming, against `crosscheck/docs/assurance-hierarchy.md` (Layer 3 row) and `cgv/README.md`.
- [ ] Leaving `/assurance-init` and `/assurance-status` unedited is right: neither makes a Layer 3 claim.
