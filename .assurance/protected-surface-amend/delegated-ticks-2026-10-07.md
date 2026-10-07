## Protected-Surface Amendment

**Target file(s):** `.claude/rules/protected-surfaces.md`, `crosscheck/skills/protected-surface-amend/SKILL.md`, `crosscheck/skills/assurance-init/SKILL.md`
**Class:** A (harness rules, skill behaviour definitions)
**Matched rule:** `.claude/rules/**`, `crosscheck/skills/*/SKILL.md`
**Date:** 2026-10-07

### Change Description

1. `.claude/rules/protected-surfaces.md`: Amendment pattern step 1 says a maintainer's explicit instruction to an agent to tick a box is a human resolution, with the comment that records who instructed it (DT-1 to DT-4). Step 4 says what resolves a marker. A new paragraph lists the mechanical checklist items that a command decides and that the drafting tool ticks with its output (DT-5, DT-6).
2. `crosscheck/skills/protected-surface-amend/SKILL.md`: the intro paragraph and the Review Checklist template draft the five mechanical items ticked with the command and its output, and emit `REQUIRES HUMAN VERIFICATION:` only for judgment items (DT-5, DT-6).
3. `crosscheck/skills/assurance-init/SKILL.md`: the Step 5 template's Amendment pattern carries the same rule, so a repository that adopts the plugin gets it (DT-7).

### Rationale

Task PB-1.38. On #108, #70 and #77 the maintainer was asked to tick boxes that a command decides ("Authoriser is a named human", "The amendment block appears in the PR body", "Diff plan enumerates every affected file"). When the maintainer then told the agent to tick them, the agent first declined, citing this rule, and the maintainer had to repeat the instruction. Intent: `intent/2026-10-07-delegated-ticks.md`. Spec: `intent/2026-10-07-delegated-ticks-spec.md`. The maintainer asked for the change in the session that handled those three pull requests.

### Governing Roadmap Item

- **Path:** `docs/assurance/ROADMAP.md` (immediate horizon, item PB-1)
- **Title:** Adopt the AI-native SDLC playbook as the development framework and make every human gate self-explanatory
- **Scope coverage:** PB-1's scope is the development framework, `REVIEW.md` and "every human gate self-explanatory". Task PB-1.38 in `docs/TASKS.md` is this change.

### Authority

- **Authoriser:** harry-nicholls. The merge is the approval.
- **Role:** Maintainer

### Diff Plan

| # | File | Section | Action |
|---|------|---------|--------|
| 1 | `.claude/rules/protected-surfaces.md` | Amendment pattern, steps 1 and 4, new paragraph | delegated ticks, mechanical items |
| 2 | `crosscheck/skills/protected-surface-amend/SKILL.md` | intro, Review Checklist template | mechanical items drafted ticked |
| 3 | `crosscheck/skills/assurance-init/SKILL.md` | Step 5 template, Amendment pattern | same rule for plugin users |

### Test / Coverage Impact

- No test reads these files, so none changes. A check that recomputes the mechanical items is not yet reached (PB-1.39).
- The guarantee gets weaker in one way: five facts move from a human's tick to a command's output that a reviewer can read. Nothing a reviewer must judge moves.
- No invariant or eval changes.

### Review Checklist

- [x] Rationale is anchored to three pull requests where the maintainer was asked to tick mechanical boxes.
- [x] Authoriser is a named human: `harry-nicholls`, not a bot or an agent.
- [x] PB-1 covers the change: it names `REVIEW.md` and the human gates.
- [x] The diff plan names every changed protected file: `git diff --name-only origin/main...HEAD` matches the three protected paths above.
- [x] No invariant is being weakened to make a test pass: no test or invariant changes.
- [ ] REQUIRES HUMAN VERIFICATION: The maintainer accepts that an agent's tick on the maintainer's instruction counts as the human's resolution (DT-1), and that the five mechanical items are no longer a human's tick (DT-5).
