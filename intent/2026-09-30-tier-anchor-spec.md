# Spec: Anchor the tier gate's `Tier:` line, and label unchecked code "not yet reached"

Intent: `intent/2026-09-30-tier-anchor.md`. Governing roadmap item: PB-1. Issue: #50.

This spec revises TG-1 and TG-8 of `intent/2026-09-29-deterministic-evidence-spec.md`. The other TG requirements do not change.

- **TG-1 (revised). Declaration and floor.** A tier comes from a `Tier: N` line or a `tier:N` label, where N is 1, 2 or 3.
  - A `Tier:` line starts with the keyword, optionally after spaces or tabs. The keyword is case-insensitive. A list or quote marker (`- `, `* `, `+ `, `> `) before the keyword makes the line not a declaration, so quoted or list-item text cannot set the tier. `Tier:` in the middle of a line does not declare a tier either.
  - The first `Tier:` line in the body is the declaration, whether or not it is valid. It is valid only when the text after `Tier:` is exactly `1`, `2` or `3`, with optional spaces around it. Anything else fails the gate: `Tier: 4`, `Tier: 1.5`, `Tier: 12`, `Tier: one`, an empty `Tier:`, `Tier: 1 (routine)`. A later valid line does not rescue an invalid first one.
  - A `Tier:` line and a `tier:N` label must agree. If the body declares one tier and a label names another, the gate fails. Two `tier:N` labels that name different tiers also fail. With no `Tier:` line, the label declares the tier.
  - A `Tier:` line inside a fenced code block still declares, if it is the first. The gate does not parse Markdown structure. This is a known gap, not a rule.
  - If the diff touches a protected path, the floor is Tier 3. A declaration below the floor fails.
  - The `Intent:`, `Spec:` and `Plan:` citations keep the TG-2 rule, which accepts a list or quote marker. A citation only has to name a file that exists, so a quoted citation cannot choose which requirements apply, and a quoted `Tier:` line could. The citation rule belongs to task PB-1.5 (#49), which widens it.
- **TG-8 (revised). Evidence report.** On pass, the gate prints one line for each class of changed file, in the order the classes first appear in the changed-file list. The report is information only and never changes the result. A class is one of two kinds:
  - *checked*: the line is `- <workflow>: <n> file(s), e.g. <first path>`;
  - *not yet reached*: the line is `- not yet reached: <n> file(s), e.g. <first path>. Blocking property: <property>. Open question: <question>.`

  No line says "none required at this tier". Prose has no deterministic check, so it is not yet reached too (`docs/VISION.md`). The classes, with the first match winning:

  | # | Changed path | Kind | Workflow, or blocking property and open question |
  |---|---|---|---|
  | 1 | `crosscheck/skills/**`, `crosscheck/agents/**`, `.claude/rules/**`, `docs/assurance/**`, and any `CLAUDE.md`, `AGENTS.md` or `REVIEW.md` | not yet reached | Property: `their behaviour is prompt text that an agent interprets`. Question: `what a replayable behavioural eval of a prompt artefact looks like` |
  | 2 | `cgv/**` | checked | `CGV CI workflow (cargo test, lake build, fixtures, statement manifest and axiom check)` |
  | 3 | `crosscheck/mcp-server/**`, `crosscheck/docs/invariants/**` | checked | `CI workflow (npm test, including the property tests)` |
  | 4 | `crosscheck/conformance/**` | checked | `CI workflow, conformance job (go vet, go test, go run . ..)` |
  | 5 | `scripts/ci/**`, `.claude/hooks/protected-surface-guard.mjs` | checked | `Tier Gate workflow (node --test scripts/ci/*.test.mjs)` |
  | 6 | `evals/**` | checked | `Incident Eval Check workflow` |
  | 7 | `docs/invariants/**` | not yet reached | Property: `no CI job maps these invariants to the tests that cover them`. Question: `which test covers each invariant, and which workflow checks that mapping` |
  | 8 | `.github/workflows/**` | not yet reached | Property: `a workflow runs only on GitHub's runners, on GitHub's events`. Question: `how to replay a workflow against recorded events before it merges` |
  | 9 | any other path ending `.md` or `.pdf` | not yet reached | Property: `prose has no executable meaning, so no check reads what it claims`. Question: `which claims in a prose document, such as cited paths and commands, a deterministic check can verify` |
  | 10 | anything else | not yet reached | Property: `no CI workflow runs a check on this path`. Question: `which deterministic check this code needs, and which workflow runs it` |

  Row 1 comes first so that an instruction file under `cgv/`, such as `cgv/CLAUDE.md`, is not reported as checked by CGV CI. Row 10 covers, among others, `logic-distribution/**`, `formal-verification/**` other than prose, `crosscheck/scripts/**`, `crosscheck/demo/**`, `crosscheck/.claude/**`, `.claude/settings.json`, `.claude/hooks/settings-snippet.json`, and root configuration such as `package.json`.
- **TG-11. Tests.** `scripts/ci/tier-gate.test.mjs` covers:
  - `Tier: 1` in the middle of a line with no label and no protected path: the gate fails and asks for a declaration (TG-1);
  - `Tier: 2` in the middle of a line with a `tier:1` label: the pass line reads `declared: 1` (TG-1);
  - `Tier: 1`, an indented `Tier: 1` (spaces or a tab), `tier: 1` in lower case, `Tier:1`, `Tier: 1` with trailing spaces, and `Tier: 1` followed by CR: each declares Tier 1 (TG-1);
  - `- Tier: 1`, `* Tier: 1`, `+ Tier: 1`, `> Tier: 1` and an indented `> Tier: 1`: each declares nothing, and `> Tier: 2` does not override a `tier:1` label (TG-1);
  - two `Tier:` lines, `Tier: 1` then `Tier: 2`: Tier 1 is declared; `> Tier: 1` then `Tier: 3`: Tier 3 is declared (TG-1);
  - an invalid first `Tier:` line fails the gate even with a `tier:2` label: `Tier: 4`, `Tier: 0`, `Tier: 1.5`, `Tier: 12`, `Tier: one`, `Tier:`, `Tier: 1 (routine)`, `Tier: 1, see below`, and `Tier: 4` then `Tier: 2` (TG-1);
  - a `Tier:` line and a label that agree pass; a line and a label that disagree fail; two labels that disagree fail (TG-1);
  - a pass whose changed files hit every class in the table: the report lines equal the literal expected lines, in order (TG-8);
  - a pass whose only change is under a row 10 path: no report line says `none required at this tier` (TG-8);
  - `CLAUDE.md`, `AGENTS.md`, `REVIEW.md` and `cgv/CLAUDE.md` each report as prompt text (TG-8);
  - a pass over prose, an instruction file, a root invariant document and configuration: every report line is `not yet reached` (TG-8).
- **DOC-6.** `docs/assurance/TIER-LAYER-MAP.md` (declaration and the evidence table) and `docs/gates/tier-layer-gate.md` (how to declare a tier, and what the pass report lists) state the revised TG-1 and TG-8, including that a line and a label must agree. `intent/2026-09-29-deterministic-evidence-spec.md` points TG-1 and TG-8 at this spec.
