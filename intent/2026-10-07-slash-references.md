# Intent: Check every slash-reference in Crosscheck's Markdown

Task: PB-1.18. Governing roadmap item: PB-1. Issue: #36. Decision: `intent/2026-10-07-backlog-review-decisions.md`, section "#36 → PB-1.18".

## Problem statement

Crosscheck's skills and agents tell the reader, often another agent, to run a skill by writing `/name` or `/<plugin>:<name>`. No check resolves these references. A renamed or deleted skill leaves every mention of it pointing at nothing, and an agent that follows the text fails or improvises. Conformance AUTO 2 resolves references in five doc files, and AUTO 5 resolves backtick references in agent bodies. Nothing covers the 30 `SKILL.md` files or the rest of `crosscheck/**/*.md`.

`crosscheck/skills/journal-context/SKILL.md:30` names `/journal-lint`, which does not exist. `crosscheck/docs/skills.md` says "all 29 skills". `crosscheck/skills/` has 30, and `journal-context` is missing from the catalogue.

## Proposed outcome

- One deterministic checker, run by the pre-commit hook and by a CI workflow, finds each `/name` and `/<plugin>:<name>` in `crosscheck/**/*.md`. It resolves an unprefixed name, or one prefixed `crosscheck:`, against `crosscheck/skills/` and `crosscheck/agents/`. It resolves a name with another plugin's prefix against an allowlist file, empty by default.
- It prints the file and line of each reference that does not resolve, and fails.
- `crosscheck/docs/skills.md` is generated from `crosscheck/skills/`, and the same checker fails when the committed file differs from the generated one.
- A committed fixture with a broken reference fails the check in a test.

## Affected users and systems

- Agents and people who follow a skill's instructions. A reference in a checked file now names a skill or agent that exists.
- Contributors who rename or delete a skill. The pre-commit hook and CI name every file that still mentions it.
- `scripts/ci/pre-commit.mjs` gains a check. A new workflow runs the checker on every pull request. Both are Class A protected surfaces.
- `crosscheck/docs/skills.md` changes from a hand-written catalogue to a generated one.
- PB-1.19 reads the same allowlist from conformance AUTO 5.

## Constraints

- No LLM, no network, and no dependencies, like the other scripts under `scripts/ci/`.
- The pre-commit hook stays under its 5 second budget (PC-7), and runs the check only when a commit stages a file the check reads.
- The checker passes on `main` once this change lands. A reference that does not resolve today is fixed in the text, not hidden.
- Exit codes follow the other gates: 0 pass, 1 findings, 2 could not read.

## Open questions

None. The spec flags two concerns: which files are not checked, and how the catalogue changes shape.
