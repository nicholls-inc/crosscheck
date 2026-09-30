# Intent: Anchor the tier gate's `Tier:` line, and label unchecked code "not yet reached"

Task: PB-1.4. Governing roadmap item: PB-1. Issue: #50.

## Problem statement
The tier gate, `scripts/ci/tier-gate.mjs`, has two faults left over from #46.

1. It reads the declared tier with `/Tier:\s*([123])\b/i` anywhere in the PR body. So quoted or pasted text such as "the old PR said Tier: 1" declares Tier 1, and it overrides a `tier:N` label. The `Intent:`, `Spec:` and `Plan:` citations already have to start their own line. `Tier:` does not.
2. On a pass, the evidence report labels every path outside its five classes "none required at this tier". That covers code that no workflow checks: `logic-distribution/**`, `formal-verification/**`, `crosscheck/scripts/*.sh`, `crosscheck/demo/**`, `crosscheck/.claude/**`, and configuration such as `.claude/settings.json`. `docs/VISION.md` requires "not yet reached", with the property that blocks it and the open question. The one "not yet reached" label the gate has, "human review is the only evidence", names neither.

The report is also wrong in the other direction for two paths. `crosscheck/conformance/**` is checked by the `conformance` job of `ci.yml` (`go vet`, `go test`, `go run . ..`), but falls to "none required". `.claude/hooks/protected-surface-guard.mjs` is tested by `scripts/ci/protected-surface-guard.test.mjs`, which the Tier Gate job runs, but is reported as "not yet reached".

## Proposed outcome
- `Tier: N` declares a tier only when it starts a line, after an optional indent and no list or quote marker. `Tier:` in the middle of a line, in a quote or in a list item is ignored.
- The first `Tier:` line is the declaration, even when it is invalid, and an invalid one fails the gate. A `Tier:` line and a `tier:N` label must agree, and the gate fails when they do not.
- Every non-prose path the gate reports falls into one of two kinds of class. A checked class names the workflow that checks it. A "not yet reached" class names the blocking property and the open question. No path is labelled "none required at this tier". Prose has no deterministic check, so it is not yet reached as well, and agent and reviewer instructions (`CLAUDE.md`, `AGENTS.md`, `REVIEW.md`) and root `docs/invariants/**` get their own not-yet-reached rows.
- `crosscheck/conformance/**` reports the `conformance` job, and the protected-surface hook reports the Tier Gate job.
- `TIER-LAYER-MAP.md` and `docs/gates/tier-layer-gate.md` state both rules.

## Affected users and systems
- Everyone who opens a pull request here: the gate reads their PR body, and they read its pass report.
- `scripts/ci/tier-gate.mjs` and `scripts/ci/tier-gate.test.mjs`.
- `docs/assurance/TIER-LAYER-MAP.md`, `docs/gates/tier-layer-gate.md`, and the TG spec `intent/2026-09-29-deterministic-evidence-spec.md`, which gains pointers to the revised TG-1 and TG-8.
- `docs/TASKS.md` and `JOURNAL.md`.

## Constraints
- The evidence report stays information only. It never changes the gate's result (TG-8).
- A `tier:N` label, the protected floor, and TG-2 to TG-7 and TG-9 do not change.
- No workflow changes. Workflow fixes are task PB-1.7.
- No dependency is added.

## Open questions
None. The maintainer settled four questions from the review of #57: an invalid first `Tier:` line fails (1a), quote and list markers do not declare (2a), all prose is not yet reached (3b), and a line and a label must agree (4b).
