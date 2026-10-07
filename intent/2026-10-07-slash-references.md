# Intent: Check every slash-reference in Crosscheck's Markdown

Task: PB-1.18. Governing roadmap item: PB-1. Issue: #36. Decision: `intent/2026-10-07-backlog-review-decisions.md`, section "#36 → PB-1.18".

## Problem statement

Crosscheck's skills and agents tell the reader, often another agent, to run a skill by writing `/name` or `/<plugin>:<name>`. No check resolves these references. A renamed or deleted skill leaves every mention of it pointing at nothing, and an agent that follows the text fails or improvises. Conformance AUTO 2 resolves references in five doc files, and AUTO 5 resolves backtick references in agent bodies. Nothing covers the 30 `SKILL.md` files or the rest of `crosscheck/**/*.md`.

`crosscheck/skills/journal-context/SKILL.md:30` names `/journal-lint`, which does not exist. `crosscheck/docs/skills.md` says "all 29 skills". `crosscheck/skills/` has 30, and `journal-context` is missing from the catalogue.

## Proposed outcome

- One deterministic checker, run by the pre-commit hook and by a CI workflow, finds each `/name` and `/<plugin>:<name>` in `crosscheck/**/*.md`, except the archives and dated records that spec SR-2 lists. It resolves an unprefixed name, or one prefixed `crosscheck:`, against `crosscheck/skills/` and `crosscheck/agents/`. It resolves a name with another plugin's prefix against an allowlist file, empty by default.
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

- **The five skipped directories.** SR-2 skips `crosscheck/docs/add/.retrospective/`, `crosscheck/.assurance/`, `crosscheck/docs/research/`, `crosscheck/docs/reports/` and `crosscheck/docs/examples/workflows/`, where the decision for #36 named all of `crosscheck/**/*.md`. Whether they stay skipped, or some of their files are rewritten, is the maintainer's call. It is the open `REQUIRES HUMAN VERIFICATION:` marker in the governance note. Behind it is a research question: how a document marks a mention as historical, so that a checker can tell it from an instruction.
- **Catalogue shape.** The generated catalogue drops the hand-written categories, trigger phrases and owners. Bringing them back needs a new frontmatter key in every `SKILL.md`, or a second source file that can drift. Nothing has decided between those or neither.
- **Overlap with conformance.** AUTO 2 and AUTO 5 still resolve references their own way. PB-1.19 aligns AUTO 5 with this grammar and allowlist, and PB-1.22 maps the allowlist and the catalogue to this workflow in the tier gate.
- **What makes the check binding.** `--no-verify` skips the hook, and the default branch requires no status checks, so a red run informs the merge but cannot block it.
- **Hardening.** The review found gaps that make no claim of this change false: references in Markdown emphasis, YAML forms the description parser does not read, paths with control characters in the output, and tests that survive mutation. PB-1.29 and PB-1.31 to PB-1.37 in `docs/TASKS.md` track them.

Settled on the pull request: the three dated records `crosscheck/JOURNAL.md`, `crosscheck/skills/JOURNAL.md` and `crosscheck/docs/specs/rationale-2026-05-11.md` keep their text, and SR-2 skips them.
