# Spec: Anchor the tier gate's `Tier:` line, and label unchecked code "not yet reached"

Intent: `intent/2026-09-30-tier-anchor.md`. Governing roadmap item: PB-1. Issue: #50.

This spec revises TG-1 and TG-8 of `intent/2026-09-29-deterministic-evidence-spec.md`. The other TG requirements do not change.

- **TG-1 (revised). Declaration and floor.** A tier comes from a `Tier: N` line or a `tier:N` label, where N is 1, 2 or 3. A `Tier:` line follows the citation rule of TG-2: it starts with the keyword, optionally indented or after one list or quote marker (`- `, `* `, `+ `, `> `), and the keyword is case-insensitive. `Tier:` in the middle of a line does not declare a tier. The first `Tier:` line in the body wins, and a body declaration wins over a label, as before. If the diff touches a protected path, the floor is Tier 3. A declaration below the floor fails.
- **TG-8 (revised). Evidence report.** On pass, the gate prints one line for each class of changed file, in the order the classes first appear in the changed-file list. The report is information only and never changes the result. A class is one of three kinds:
  - *checked*: the line is `- <workflow>: <n> file(s), e.g. <first path>`;
  - *prose*: the line is `- none required at this tier: <n> file(s), e.g. <first path>`;
  - *not yet reached*: the line is `- not yet reached: <n> file(s), e.g. <first path>. Blocking property: <property>. Open question: <question>.`

  The classes, with the first match winning:

  | # | Changed path | Kind | Workflow, or blocking property and open question |
  |---|---|---|---|
  | 1 | `cgv/**` | checked | `CGV CI workflow (cargo test, lake build, fixtures, statement manifest and axiom check)` |
  | 2 | `crosscheck/mcp-server/**`, `crosscheck/docs/invariants/**` | checked | `CI workflow (npm test, including the property tests)` |
  | 3 | `crosscheck/conformance/**` | checked | `CI workflow, conformance job (go vet, go test, go run . ..)` |
  | 4 | `scripts/ci/**`, `.claude/hooks/protected-surface-guard.mjs` | checked | `Tier Gate workflow (node --test scripts/ci/*.test.mjs)` |
  | 5 | `evals/**` | checked | `Incident Eval Check workflow` |
  | 6 | `crosscheck/skills/**`, `crosscheck/agents/**`, `.claude/rules/**`, `docs/assurance/**` | not yet reached | Property: `their behaviour is prompt text that an agent interprets`. Question: `what a replayable behavioural eval of a prompt artefact looks like` |
  | 7 | `.github/workflows/**` | not yet reached | Property: `a workflow runs only on GitHub's runners, on GitHub's events`. Question: `how to replay a workflow against recorded events before it merges` |
  | 8 | any other path ending `.md` or `.pdf` | prose | |
  | 9 | anything else | not yet reached | Property: `no CI workflow runs a check on this path`. Question: `which deterministic check this code needs, and which workflow runs it` |

  Row 9 covers, among others, `logic-distribution/**`, `formal-verification/**` other than prose, `crosscheck/scripts/**`, `crosscheck/demo/**`, `crosscheck/.claude/**`, `.claude/settings.json`, `.claude/hooks/settings-snippet.json`, and root configuration such as `package.json`.
- **TG-11. Tests.** `scripts/ci/tier-gate.test.mjs` covers:
  - `Tier: 1` in the middle of a line with no label and no protected path: the gate fails and asks for a declaration (TG-1);
  - `Tier: 2` in the middle of a line with a `tier:1` label: the pass line reads `declared: 1` (TG-1);
  - `- Tier: 1`, `> Tier: 1`, an indented `Tier: 1`, `tier: 1` in lower case, and `Tier: 1` followed by CR: each declares Tier 1 (TG-1);
  - two `Tier:` lines, `Tier: 1` then `Tier: 2`: Tier 1 is declared (TG-1);
  - a pass whose changed files hit every class in the table: the report lines equal the literal expected lines, in order (TG-8);
  - a pass whose only non-prose change is under a row 9 path: no report line says `none required at this tier` (TG-8).
- **DOC-6.** `docs/assurance/TIER-LAYER-MAP.md` (declaration and the evidence table) and `docs/gates/tier-layer-gate.md` (how to declare a tier, and what the pass report lists) state the revised TG-1 and TG-8. `intent/2026-09-29-deterministic-evidence-spec.md` points TG-1 and TG-8 at this spec.
