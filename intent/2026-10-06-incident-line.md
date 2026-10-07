# Intent: Count only a whole `Fixes-Incident:` line as an incident reference

Task: PB-1.16. Governing roadmap item: PB-1.

## Problem statement
The Incident Eval Check (`scripts/ci/incident-eval-check.mjs`) finds an incident id with `/Fixes-Incident:\s*(\S+)/i`. The pattern matches anywhere in the text, so prose that quotes or mentions the trigger fires the check. The run for #62 (run 37527444185) failed after the merge with this report:

```
- No eval under evals/ names or references incident "<id>`". ...
```

#62 fixed no incident. Its body quoted the trigger inside a list item, in the line ``- Trigger: the `incident` label, or the text `Fixes-Incident: <id>` in any case, anywhere in a line of the PR body or a commit message.`` One of its commits wrapped a sentence so that a line began with the trigger: `Fixes-Incident: line, needs an eval and a candidate invariant, exits 2 when`. Either one alone fires the check. The body is read first, so the id became `` <id>` ``.

A pull request that explains the check, or any prose that names the trigger, therefore ends in a red run after the merge. The maintainer has to read the log to see that no incident was involved.

## Proposed outcome
- A line is an incident reference only when the whole line is the trigger and one id: optional leading spaces or tabs, `Fixes-Incident:` in any case, optional spaces or tabs, one id, and nothing after it but spaces or tabs. The id keeps today's rule: one trailing `.`, `,` or `;` is dropped.
- The rule is the same for every line of the PR body and of every commit message. The body is split into lines first, so a trigger line with no value no longer takes the next line's first word as its id.
- A list or quote marker before the trigger (`- `, `* `, `+ `, `> `) means the line is not a reference. This is the rule the tier gate already uses for its `Tier:` line and the task queue check for its `Task:` line.
- With the bodies and commits of #62, the check prints `no incident reference — skipped` and exits 0. A test replays both lines.
- `docs/assurance/DEVELOPMENT-FRAMEWORK.md` stage 5 and `docs/gates/tier-layer-gate.md` describe the new rule in place of "anywhere in a line".

## Affected users and systems
- The maintainer, who reads the check's result on each merge.
- Authors who fix an incident. A reference written as a sentence (`Fixes-Incident: INC-7 and INC-8`, or `Fixes-Incident: INC-7 (the outage)`) no longer counts. They write one `Fixes-Incident: <id>` line per incident, or set the `incident` label, which still forces the check to apply.
- `scripts/ci/incident-eval-check.mjs`, `scripts/ci/incident-eval-check.test.mjs`, `docs/assurance/DEVELOPMENT-FRAMEWORK.md` and `docs/gates/tier-layer-gate.md`. The first three are protected surfaces, so the change is Tier 3.

## Constraints
- The `incident` label, the eval and invariant lookups, and exit codes 0, 1 and 2 keep their meaning.
- The rule matches the anchor of the tier gate's `Tier:` line and the queue check's `Task:` line, so the repository has one rule for "a line that declares something".
- No CI job calls an LLM.
- PR #66 (PB-1.12) edits the same script, its tests and stage 5, and adds `docs/gates/incident-eval-check.md`, which also says "anywhere in a line". Whichever pull request merges second reconciles those lines. This change claims requirement ID IE-9, because #66 claims IE-8.

## Open questions
None. The task row asks the implementer to decide which lines count. The decision follows the repository's existing anchor rule, and the residual cases are listed in the spec.
